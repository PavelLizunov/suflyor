import Foundation
import Testing
@testable import SuflyorMLXCore

private let validBearer = String(repeating: "a1", count: 32)

private let lfmTemplateSeam =
    "{%- if add_generation_prompt -%}\n"
    + "    {{- \"<|im_start|>assistant\\n\" -}}\n"
    + "{%- endif -%}"

@Test func lfmTemplatePatchIsExact() throws {
    let source = "before\n" + lfmTemplateSeam + "\nafter"
    let expected =
        "before\n{%- if add_generation_prompt -%}\n"
        + "    {{- \"<|im_start|>assistant\\n\" -}}\n"
        + "    {%- if enable_thinking is defined and enable_thinking is false -%}\n"
        + "        {{- \"<think>\\nNo unnecessary reasoning. Close thinking and answer immediately.\\n</think>\\n\" -}}\n"
        + "    {%- endif -%}\n"
        + "{%- endif -%}\nafter"

    #expect(try patchLFMChatTemplate(source) == expected)
}

@Test func lfmTemplatePatchRequiresOneExactSeam() {
    #expect(throws: SidecarError.invalidSnapshot) {
        try patchLFMChatTemplate(lfmTemplateSeam + "\n" + lfmTemplateSeam)
    }
    #expect(throws: SidecarError.invalidSnapshot) {
        try patchLFMChatTemplate(lfmTemplateSeam.replacingOccurrences(of: " -%}", with: " %}"))
    }
}

@Test func lfmTemplatePatchRejectsAlreadyPatchedOrUnknownTemplates() throws {
    let patched = try patchLFMChatTemplate(lfmTemplateSeam)
    #expect(throws: SidecarError.invalidSnapshot) { try patchLFMChatTemplate(patched) }
    #expect(throws: SidecarError.invalidSnapshot) {
        try patchLFMChatTemplate("{%- if add_generation_prompt -%}unknown{%- endif -%}")
    }
}

@Test func generationDiagnosticsAreBoundedTechnicalTokens() {
    let token = generationDiagnosticToken(String(repeating: "Type/путь\n", count: 32))
    #expect(token.count == 96)
    #expect(token.utf8.allSatisfy { byte in
        (48...57).contains(byte) || (65...90).contains(byte) || (97...122).contains(byte)
            || byte == 45 || byte == 46 || byte == 95
    })
    #expect(!token.contains("путь"))
    #expect(!token.contains("/"))
    #expect(!token.contains("\n"))
    #expect(sidecarDiagnosticCase(SidecarError.invalidSnapshot) == "invalid_snapshot")
    #expect(sidecarDiagnosticCase(SidecarError.reasoningBoundaryMissing) == "reasoning_boundary_missing")
    #expect(sidecarDiagnosticCase(SidecarError.generationIncomplete) == "generation_incomplete")
    #expect(sidecarDiagnosticCase(CancellationError()) == "none")
}

private func startupData(
    version: Int = StartupConfiguration.protocolVersion,
    bearer: String = validBearer,
    model: String = SupportedModel.lfm.rawValue,
    snapshot: String = "/tmp/suflyor-model"
) throws -> Data {
    try JSONSerialization.data(withJSONObject: [
        "version": version,
        "bearer": bearer,
        "model": model,
        "snapshot": snapshot,
    ])
}

private func firstImageData(in request: ChatCompletionRequest) throws -> Data {
    guard let message = request.messages.first,
          case .parts(let parts) = message.content,
          let part = parts.first,
          case .image(let data) = part
    else { throw SidecarError.invalidRequest }
    return data
}

@Test func startupIsPinnedToLoopbackEphemeralAndSelectedModel() throws {
    let startup = try StartupConfiguration(data: startupData())
    #expect(StartupConfiguration.host == "127.0.0.1")
    #expect(StartupConfiguration.port == 0)
    #expect(startup.bearer == validBearer)
    #expect(startup.model == .lfm)
    #expect(startup.snapshot.path == "/tmp/suflyor-model")
    #expect(startup.accepts(.lfm))
    #expect(!startup.accepts(.qwen))
}

@Test func startupRejectsMalformedOrUntrustedWireValues() throws {
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: Data())
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: startupData(version: 2))
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: startupData(model: "unsupported/model"))
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: startupData(bearer: String(repeating: "a", count: 63)))
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: startupData(bearer: String(repeating: "A", count: 64)))
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: startupData(bearer: String(repeating: "g", count: 64)))
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: startupData(snapshot: "relative/model"))
    }
    #expect(throws: SidecarError.invalidStartup) {
        try StartupConfiguration(data: Data(repeating: 0x20, count: StartupConfiguration.maxWireBytes + 1))
    }
}

@Test func bearerCheckIsExact() {
    #expect(isAuthorized("Bearer \(validBearer)", bearer: validBearer))
    #expect(!isAuthorized("Bearer \(validBearer)a", bearer: validBearer))
    #expect(!isAuthorized(validBearer, bearer: validBearer))
    #expect(!isAuthorized(nil, bearer: validBearer))
}

@Test func parsesTextAndOnlyAcceptsPngOrJpegDataURLs() throws {
    let text = Data(#"{"model":"LiquidAI/LFM2.5-8B-A1B-MLX-4bit","messages":[{"role":"user","content":"hello"}]}"#.utf8)
    let request = try JSONDecoder().decode(ChatCompletionRequest.self, from: text)
    #expect(request.model == .lfm)
    #expect(!request.stream)

    #expect(try decodeImageDataURL("data:image/jpeg;base64,AA==") == Data([0]))
    #expect(try decodeImageDataURL("data:image/png;base64,AQ==") == Data([1]))
    let jpeg = Data(#"{"model":"mlx-community/Qwen3.5-2B-4bit","messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":"data:image/jpeg;base64,AA=="}}]}]}"#.utf8)
    let png = Data(#"{"model":"mlx-community/Qwen3.5-2B-4bit","messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":"data:image/png;base64,AQ=="}}]}]}"#.utf8)
    #expect(try firstImageData(in: JSONDecoder().decode(ChatCompletionRequest.self, from: jpeg)) == Data([0]))
    #expect(try firstImageData(in: JSONDecoder().decode(ChatCompletionRequest.self, from: png)) == Data([1]))

    let remote = Data(#"{"model":"mlx-community/Qwen3.5-2B-4bit","messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":"https://example.invalid/a.jpg"}}]}]}"#.utf8)
    #expect(throws: SidecarError.invalidRequest) {
        try JSONDecoder().decode(ChatCompletionRequest.self, from: remote)
    }
    #expect(throws: SidecarError.invalidRequest) {
        try decodeImageDataURL("data:image/gif;base64,AA==")
    }
    #expect(throws: SidecarError.invalidRequest) {
        try decodeImageDataURL("data:image/png;base64,")
    }
    #expect(throws: SidecarError.invalidRequest) {
        try decodeImageDataURL("data:image/jpeg;base64,not-base64")
    }
}

@Test func sseAndReasoningAreFailClosed() throws {
    #expect(sseData(Data(#"{"x":1}"#.utf8)) == "data: {\"x\":1}\n\n")
    var filter = ReasoningFilter(required: true)
    #expect(try filter.feed("<think>secret") == "")
    #expect(try filter.feed("</think>answer", finished: true) == "answer")
    var disabled = ReasoningFilter(required: false)
    #expect(try disabled.feed("<think>unexpected</think>answer", finished: true) == "answer")
    var truncatedOptional = ReasoningFilter(required: false)
    #expect(try truncatedOptional.feed("<think>unfinished", finished: true) == "")
    var ordinary = ReasoningFilter(required: SupportedModel.lfm.requiresReasoningBoundary)
    #expect(try ordinary.feed("plain answer", finished: true) == "plain answer")
    var streaming = ReasoningFilter(required: false)
    #expect(try streaming.feed("first") == "first")
    var requiredStreaming = ReasoningFilter(required: true)
    #expect(try requiredStreaming.feed("<think>secret</thi") == "")
    #expect(try requiredStreaming.feed("nk>first") == "first")
    #expect(try requiredStreaming.feed(" answer") == " answer")
    var incomplete = ReasoningFilter(required: true)
    _ = try incomplete.feed("secret")
    #expect(throws: SidecarError.reasoningBoundaryMissing) {
        try incomplete.feed("", finished: true)
    }
}

@Test func completionMetricsExposeOpenAIUsageAndDecodeTPS() throws {
    let metrics = completionMetrics(
        promptTokenCount: 12,
        generationTokenCount: 30,
        generateTime: 2.0
    )
    #expect(metrics.usage == CompletionUsage(
        promptTokens: 12,
        completionTokens: 30,
        totalTokens: 42
    ))
    #expect(metrics.timings == CompletionTimings(predictedPerSecond: 15.0))

    let usage = try JSONSerialization.jsonObject(with: JSONEncoder().encode(metrics.usage))
        as? [String: Int]
    let timings = try JSONSerialization.jsonObject(with: JSONEncoder().encode(metrics.timings))
        as? [String: Double]
    #expect(usage?["prompt_tokens"] == 12)
    #expect(usage?["completion_tokens"] == 30)
    #expect(usage?["total_tokens"] == 42)
    #expect(timings?["predicted_per_second"] == 15.0)

    let empty = completionMetrics(promptTokenCount: 0, generationTokenCount: 0, generateTime: 0)
    #expect(empty.timings.predictedPerSecond == nil)
}

@Test func cancelledGenerationIsNeverASuccessfulStop() {
    #expect(throws: CancellationError.self) {
        try openAIFinishReason(.cancelled)
    }
}

@Test func readyLineCarriesProtocolAndResidentModel() throws {
    let line = try readyLine(port: 49_151, model: .qwen)
    #expect(line.last == 0x0A)
    let object = try #require(
        JSONSerialization.jsonObject(with: Data(line.dropLast())) as? [String: Any]
    )
    #expect(object["event"] as? String == "READY")
    #expect(object["version"] as? Int == StartupConfiguration.protocolVersion)
    #expect(object["port"] as? Int == 49_151)
    #expect(object["model"] as? String == SupportedModel.qwen.rawValue)
    #expect(throws: SidecarError.invalidStartup) { try readyLine(port: 0, model: .qwen) }
    #expect(throws: SidecarError.invalidStartup) { try readyLine(port: 65_536, model: .qwen) }

    let gemmaLine = try readyLine(port: 49_152, model: .gemma4)
    let gemmaObj = try #require(
        JSONSerialization.jsonObject(with: Data(gemmaLine.dropLast())) as? [String: Any]
    )
    #expect(gemmaObj["model"] as? String == SupportedModel.gemma4.rawValue)
    #expect(SupportedModel.gemma4.modelType == "gemma4")
    #expect(SupportedModel.gemma4.supportsVision == true)
}

@Test func cancellingQueuedGenerationDoesNotStrandGate() async throws {
    let gate = GenerationGate()
    try await gate.acquire()
    let waiting = Task { try await gate.acquire() }
    waiting.cancel()

    var cancelled = false
    do {
        try await waiting.value
    } catch is CancellationError {
        cancelled = true
    }
    #expect(cancelled)

    await gate.release()
    try await gate.acquire()
    await gate.release()
}
