import Foundation
import Hummingbird
import MLXHuggingFace
import MLXLLM
import MLXLMCommon
import MLXVLM
import Tokenizers

struct CompletionUsage: Encodable, Equatable, Sendable {
    let promptTokens: Int
    let completionTokens: Int
    let totalTokens: Int

    enum CodingKeys: String, CodingKey {
        case promptTokens = "prompt_tokens"
        case completionTokens = "completion_tokens"
        case totalTokens = "total_tokens"
    }
}

struct CompletionTimings: Encodable, Equatable, Sendable {
    let predictedPerSecond: Double?

    enum CodingKeys: String, CodingKey {
        case predictedPerSecond = "predicted_per_second"
    }
}

func completionMetrics(
    promptTokenCount: Int,
    generationTokenCount: Int,
    generateTime: TimeInterval
) -> (usage: CompletionUsage, timings: CompletionTimings) {
    let rate = generateTime > 0 ? Double(generationTokenCount) / generateTime : nil
    let finiteRate = rate.flatMap { $0.isFinite && $0 > 0 ? $0 : nil }
    return (
        CompletionUsage(
            promptTokens: promptTokenCount,
            completionTokens: generationTokenCount,
            totalTokens: promptTokenCount + generationTokenCount
        ),
        CompletionTimings(predictedPerSecond: finiteRate)
    )
}

func openAIFinishReason(_ reason: GenerateStopReason) throws -> String {
    switch reason {
    case .length: "length"
    case .stop: "stop"
    case .cancelled: throw CancellationError()
    }
}

private struct CompletionEnvelope: Encodable {
    struct Choice: Encodable {
        let index = 0
        let message: Message
        let finishReason: String
        enum CodingKeys: String, CodingKey { case index, message; case finishReason = "finish_reason" }
    }
    struct Message: Encodable { let role = "assistant"; let content: String }
    let id: String
    let object = "chat.completion"
    let model: String
    let choices: [Choice]
    let usage: CompletionUsage
    let timings: CompletionTimings
}

private struct ChunkEnvelope: Encodable {
    struct Choice: Encodable {
        let index = 0
        let delta: Delta
        let finishReason: String?
        enum CodingKeys: String, CodingKey { case index, delta; case finishReason = "finish_reason" }
    }
    struct Delta: Encodable { let content: String? }
    let id: String
    let object = "chat.completion.chunk"
    let model: String
    let choices: [Choice]
    let usage: CompletionUsage?
    let timings: CompletionTimings?
}

private struct ModelList: Encodable {
    struct Model: Encodable { let id: String; let object = "model" }
    let object = "list"
    let data: [Model]
}

private struct ReadyEnvelope: Encodable {
    let event = "READY"
    let version = StartupConfiguration.protocolVersion
    let port: Int
    let model: String
}

private enum GenerationPhase: String {
    case container
    case userInput = "user_input"
    case prepare
    case generate
    case iterate
    case reasoningFilter = "reasoning_filter"
}

func generationDiagnosticToken(_ value: String) -> String {
    let allowed = Set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-")
    return String(value.map { allowed.contains($0) ? $0 : "_" }.prefix(96))
}

private let lfmGenerationPromptSeam =
    "{%- if add_generation_prompt -%}\n"
    + "    {{- \"<|im_start|>assistant\\n\" -}}\n"
    + "{%- endif -%}"

private let lfmNoThinkGenerationPrompt =
    "{%- if add_generation_prompt -%}\n"
    + "    {{- \"<|im_start|>assistant\\n\" -}}\n"
    + "    {%- if enable_thinking is defined and enable_thinking is false -%}\n"
    + "        {{- \"<think>\\nNo unnecessary reasoning. Close thinking and answer immediately.\\n</think>\\n\" -}}\n"
    + "    {%- endif -%}\n"
    + "{%- endif -%}"

func patchLFMChatTemplate(_ template: String) throws -> String {
    guard template.components(separatedBy: lfmGenerationPromptSeam).count == 2 else {
        throw SidecarError.invalidSnapshot
    }
    return template.replacingOccurrences(
        of: lfmGenerationPromptSeam,
        with: lfmNoThinkGenerationPrompt
    )
}

private struct LFMNoThinkTokenizerLoader: TokenizerLoader {
    let base: any TokenizerLoader

    func load(from directory: URL) async throws -> any MLXLMCommon.Tokenizer {
        let fileManager = FileManager.default
        let overlay = fileManager.temporaryDirectory
            .appendingPathComponent("suflyor-lfm-tokenizer-\(UUID().uuidString)", isDirectory: true)
        do {
            let templateURL = directory.appendingPathComponent("chat_template.jinja")
            let template = try String(contentsOf: templateURL, encoding: .utf8)
            let patched = try patchLFMChatTemplate(template)
            try fileManager.createDirectory(
                at: overlay,
                withIntermediateDirectories: false,
                attributes: [.posixPermissions: 0o700]
            )
            defer { try? fileManager.removeItem(at: overlay) }
            for entry in try fileManager.contentsOfDirectory(
                at: directory,
                includingPropertiesForKeys: nil
            ) where entry.lastPathComponent != "chat_template.jinja" {
                try fileManager.createSymbolicLink(
                    at: overlay.appendingPathComponent(entry.lastPathComponent),
                    withDestinationURL: entry
                )
            }
            try patched.write(
                to: overlay.appendingPathComponent("chat_template.jinja"),
                atomically: true,
                encoding: .utf8
            )
            return try await base.load(from: overlay)
        } catch {
            throw SidecarError.invalidSnapshot
        }
    }
}

func sidecarDiagnosticCase(_ error: Error) -> String {
    guard let error = error as? SidecarError else { return "none" }
    switch error {
    case .invalidStartup: return "invalid_startup"
    case .unauthorized: return "unauthorized"
    case .invalidRequest: return "invalid_request"
    case .unsupportedModel: return "unsupported_model"
    case .invalidSnapshot: return "invalid_snapshot"
    case .reasoningBoundaryMissing: return "reasoning_boundary_missing"
    case .generationIncomplete: return "generation_incomplete"
    }
}

private func logFailure(scope: String, phase: String, error: Error) {
    let nsError = error as NSError
    let type = generationDiagnosticToken(String(reflecting: Swift.type(of: error)))
    let domain = generationDiagnosticToken(nsError.domain)
    let sidecarCase = sidecarDiagnosticCase(error)
    let line = "MLX failure scope=\(scope) phase=\(phase) type=\(type) domain=\(domain) code=\(nsError.code) sidecar=\(sidecarCase)\n"
    try? FileHandle.standardError.write(contentsOf: Data(line.utf8))
}

func readyLine(port: Int, model: SupportedModel) throws -> Data {
    guard (1...65_535).contains(port) else { throw SidecarError.invalidStartup }
    var data = try JSONEncoder().encode(ReadyEnvelope(port: port, model: model.rawValue))
    data.append(0x0A)
    return data
}

actor GenerationGate {
    private struct Waiter {
        let id: UUID
        let continuation: CheckedContinuation<Void, Error>
    }

    private var busy = false
    private var waiters: [Waiter] = []

    func acquire() async throws {
        try Task.checkCancellation()
        guard busy else {
            busy = true
            return
        }

        let id = UUID()
        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation {
                (continuation: CheckedContinuation<Void, Error>) in
                if Task.isCancelled {
                    continuation.resume(throwing: CancellationError())
                } else {
                    waiters.append(.init(id: id, continuation: continuation))
                }
            }
        } onCancel: {
            Task { await self.cancel(id) }
        }
    }

    func release() {
        if waiters.isEmpty {
            busy = false
        } else {
            waiters.removeFirst().continuation.resume()
        }
    }

    private func cancel(_ id: UUID) {
        guard let index = waiters.firstIndex(where: { $0.id == id }) else { return }
        waiters.remove(at: index).continuation.resume(throwing: CancellationError())
    }
}

private enum GenerationOutput: Sendable {
    case chunk(String)
    case info(GenerateCompletionInfo)
}

private actor ModelEngine {
    private let model: SupportedModel
    private let snapshot: URL
    private let gate = GenerationGate()
    private var loaded: ModelContainer?

    init(model: SupportedModel, snapshot: URL) {
        self.model = model
        self.snapshot = snapshot
    }

    func preload() async throws {
        let container = try await container()
        try Task.checkCancellation()
        do {
            let warmupInput = UserInput(prompt: "1")
            let prepared = try await container.prepare(input: warmupInput)
            let stream = try await container.generate(
                input: prepared,
                parameters: .init(maxTokens: 1, temperature: 0.0)
            )
            for await _ in stream {}
        } catch {
            // Non-fatal warmup failure: real requests will proceed normally.
        }
        try Task.checkCancellation()
    }

    nonisolated func events(
        for request: ChatCompletionRequest
    ) -> AsyncThrowingStream<GenerationOutput, Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    try await self.generate(request: request) { _ = continuation.yield($0) }
                    continuation.finish()
                } catch let error as SidecarError {
                    continuation.finish(throwing: error)
                } catch is CancellationError {
                    continuation.finish(throwing: CancellationError())
                } catch {
                    continuation.finish(throwing: SidecarError.invalidSnapshot)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    func generate(
        request: ChatCompletionRequest,
        onEvent: @Sendable (GenerationOutput) async throws -> Void
    ) async throws {
        guard request.model == model else { throw SidecarError.unsupportedModel }
        try await gate.acquire()
        var phase = GenerationPhase.container
        do {
            try Task.checkCancellation()
            let container = try await container()
            try Task.checkCancellation()
            phase = .userInput
            let input = try request.userInput()
            phase = .prepare
            let prepared = try await container.prepare(input: input)
            try Task.checkCancellation()
            phase = .generate
            let stream = try await container.generate(
                input: prepared,
                parameters: .init(
                    maxTokens: request.maxTokens ?? 2048,
                    temperature: request.temperature ?? 0.2
                )
            )
            try Task.checkCancellation()
            var filter = ReasoningFilter(required: request.model.requiresReasoningBoundary)
            var completionInfo: GenerateCompletionInfo?
            phase = .iterate
            for await event in stream {
                try Task.checkCancellation()
                switch event {
                case .chunk(let text):
                    phase = .reasoningFilter
                    let visible = try filter.feed(text)
                    if !visible.isEmpty { try await onEvent(.chunk(visible)) }
                    phase = .iterate
                case .info(let info):
                    completionInfo = info
                case .toolCall:
                    break
                }
            }
            guard let completionInfo else { throw SidecarError.generationIncomplete }
            if case .cancelled = completionInfo.stopReason { throw CancellationError() }
            phase = .reasoningFilter
            let tail = try filter.feed("", finished: true)
            if !tail.isEmpty { try await onEvent(.chunk(tail)) }
            try await onEvent(.info(completionInfo))
            await gate.release()
        } catch {
            await gate.release()
            if !(error is CancellationError) {
                logFailure(scope: "generation", phase: phase.rawValue, error: error)
            }
            throw error
        }
    }

    private func container() async throws -> ModelContainer {
        if let loaded { return loaded }
        try validateSnapshot(snapshot, model: model)
        let tokenizer = #huggingFaceTokenizerLoader()
        let next: ModelContainer
        do {
            switch model {
            case .lfm:
                next = try await LLMModelFactory.shared.loadContainer(
                    from: snapshot,
                    using: LFMNoThinkTokenizerLoader(base: tokenizer)
                )
            case .qwen, .gemma4:
                next = try await VLMModelFactory.shared.loadContainer(from: snapshot, using: tokenizer)
            }
        } catch {
            throw SidecarError.invalidSnapshot
        }
        try Task.checkCancellation()
        loaded = next
        return next
    }

    private func validateSnapshot(_ directory: URL, model: SupportedModel) throws {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: directory.path, isDirectory: &isDirectory),
              isDirectory.boolValue,
              let data = try? Data(contentsOf: directory.appendingPathComponent("config.json")),
              let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              object["model_type"] as? String == model.modelType
        else { throw SidecarError.invalidSnapshot }
    }
}

public struct SidecarServer: Sendable {
    private let startup: StartupConfiguration
    public init(startup: StartupConfiguration) { self.startup = startup }

    public func run() async throws {
        let engine = ModelEngine(model: startup.model, snapshot: startup.snapshot)
        let bearer = startup.bearer
        let router = Router()

        router.get("/health") { request, context -> Response in
            try authorize(request, bearer: bearer)
            return try context.responseEncoder.encode(["status": "ok"], from: request, context: context)
        }
        router.get("/v1/models") { request, context -> Response in
            try authorize(request, bearer: bearer)
            let models = ModelList(data: [.init(id: startup.model.rawValue)])
            return try context.responseEncoder.encode(models, from: request, context: context)
        }
        router.post("/v1/chat/completions") { request, context -> Response in
            try authorize(request, bearer: bearer)
            let completion = try await request.decode(as: ChatCompletionRequest.self, context: context)
            guard startup.accepts(completion.model) else { throw HTTPError(.badRequest) }
            let id = "chatcmpl-\(UUID().uuidString.lowercased())"
            if completion.stream {
                return Response(
                    status: .ok,
                    headers: [.contentType: "text/event-stream", .cacheControl: "no-cache"],
                    body: .init { writer in
                        let encoder = JSONEncoder()
                        var completionInfo: GenerateCompletionInfo?
                        for try await event in engine.events(for: completion) {
                            switch event {
                            case .chunk(let text):
                                let chunk = ChunkEnvelope(
                                    id: id, model: completion.model.rawValue,
                                    choices: [.init(delta: .init(content: text), finishReason: nil)],
                                    usage: nil, timings: nil
                                )
                                try await writer.write(ByteBuffer(string: sseData(try encoder.encode(chunk))))
                            case .info(let info):
                                completionInfo = info
                            }
                        }
                        guard let completionInfo else { throw SidecarError.generationIncomplete }
                        let metrics = completionMetrics(
                            promptTokenCount: completionInfo.promptTokenCount,
                            generationTokenCount: completionInfo.generationTokenCount,
                            generateTime: completionInfo.generateTime
                        )
                        let end = ChunkEnvelope(
                            id: id, model: completion.model.rawValue,
                            choices: [.init(
                                delta: .init(content: nil),
                                finishReason: try openAIFinishReason(completionInfo.stopReason)
                            )],
                            usage: metrics.usage,
                            timings: metrics.timings
                        )
                        try await writer.write(ByteBuffer(string: sseData(try encoder.encode(end))))
                        try await writer.write(ByteBuffer(string: "data: [DONE]\n\n"))
                        try await writer.finish(nil)
                    }
                )
            }
            var answer = ""
            var completionInfo: GenerateCompletionInfo?
            for try await event in engine.events(for: completion) {
                switch event {
                case .chunk(let text): answer += text
                case .info(let info): completionInfo = info
                }
            }
            guard let completionInfo else { throw SidecarError.generationIncomplete }
            let metrics = completionMetrics(
                promptTokenCount: completionInfo.promptTokenCount,
                generationTokenCount: completionInfo.generationTokenCount,
                generateTime: completionInfo.generateTime
            )
            let envelope = CompletionEnvelope(
                id: id, model: completion.model.rawValue,
                choices: [.init(
                    message: .init(content: answer),
                    finishReason: try openAIFinishReason(completionInfo.stopReason)
                )],
                usage: metrics.usage,
                timings: metrics.timings
            )
            return try context.responseEncoder.encode(envelope, from: request, context: context)
        }

        let app = Application(
            router: router,
            configuration: .init(address: .hostname(StartupConfiguration.host, port: StartupConfiguration.port)),
            onServerRunning: { channel in
                guard let port = channel.localAddress?.port else { return }
                guard let ready = try? readyLine(port: port, model: startup.model) else { return }
                try? FileHandle.standardOutput.write(contentsOf: ready)
            }
        )

        try await withThrowingTaskGroup(of: Void.self) { group in
            group.addTask {
                try await engine.preload()
                try Task.checkCancellation()
                try await app.runService(gracefulShutdownSignals: [])
            }
            group.addTask {
                for try await _ in FileHandle.standardInput.bytes {}
            }
            _ = try await group.next()
            group.cancelAll()
        }
    }
}

private func authorize(_ request: Request, bearer: String) throws {
    guard isAuthorized(request.headers[.authorization], bearer: bearer) else {
        throw HTTPError(.unauthorized)
    }
}
