import Foundation
import Hummingbird
import MLXHuggingFace
import MLXLLM
import MLXLMCommon
import MLXVLM
import Tokenizers

private struct CompletionEnvelope: Encodable {
    struct Choice: Encodable {
        let index = 0
        let message: Message
        let finishReason = "stop"
        enum CodingKeys: String, CodingKey { case index, message; case finishReason = "finish_reason" }
    }
    struct Message: Encodable { let role = "assistant"; let content: String }
    let id: String
    let object = "chat.completion"
    let model: String
    let choices: [Choice]
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

private func logGenerationFailure(phase: GenerationPhase, error: Error) {
    let nsError = error as NSError
    let type = generationDiagnosticToken(String(reflecting: Swift.type(of: error)))
    let domain = generationDiagnosticToken(nsError.domain)
    let line = "MLX generation failed phase=\(phase.rawValue) type=\(type) domain=\(domain) code=\(nsError.code)\n"
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
        _ = try await container()
        try Task.checkCancellation()
    }

    nonisolated func events(for request: ChatCompletionRequest) -> AsyncThrowingStream<String, Error> {
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
        onChunk: @Sendable (String) async throws -> Void
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
            phase = .iterate
            for await event in stream {
                try Task.checkCancellation()
                if case .chunk(let text) = event {
                    phase = .reasoningFilter
                    let visible = try filter.feed(text)
                    if !visible.isEmpty { try await onChunk(visible) }
                    phase = .iterate
                }
            }
            phase = .reasoningFilter
            let tail = try filter.feed("", finished: true)
            if !tail.isEmpty { try await onChunk(tail) }
            await gate.release()
        } catch {
            await gate.release()
            if !(error is SidecarError), !(error is CancellationError) {
                logGenerationFailure(phase: phase, error: error)
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
                next = try await LLMModelFactory.shared.loadContainer(from: snapshot, using: tokenizer)
            case .qwen:
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
                        for try await text in engine.events(for: completion) {
                            let chunk = ChunkEnvelope(
                                id: id, model: completion.model.rawValue,
                                choices: [.init(delta: .init(content: text), finishReason: nil)]
                            )
                            try await writer.write(ByteBuffer(string: sseData(try encoder.encode(chunk))))
                        }
                        let end = ChunkEnvelope(
                            id: id, model: completion.model.rawValue,
                            choices: [.init(delta: .init(content: nil), finishReason: "stop")]
                        )
                        try await writer.write(ByteBuffer(string: sseData(try encoder.encode(end))))
                        try await writer.write(ByteBuffer(string: "data: [DONE]\n\n"))
                        try await writer.finish(nil)
                    }
                )
            }
            var answer = ""
            for try await text in engine.events(for: completion) { answer += text }
            let envelope = CompletionEnvelope(
                id: id, model: completion.model.rawValue,
                choices: [.init(message: .init(content: answer))]
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
