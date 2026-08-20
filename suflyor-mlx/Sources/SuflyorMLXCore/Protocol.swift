import CoreImage
import Foundation
import MLXLMCommon

public enum SidecarError: Error, Equatable {
    case invalidStartup
    case unauthorized
    case invalidRequest
    case unsupportedModel
    case invalidSnapshot
    case reasoningBoundaryMissing
}

public enum SupportedModel: String, CaseIterable, Codable, Sendable {
    case lfm = "LiquidAI/LFM2.5-8B-A1B-MLX-4bit"
    case qwen = "mlx-community/Qwen3.5-2B-4bit"

    public var modelType: String {
        switch self {
        case .lfm: "lfm2_moe"
        case .qwen: "qwen3_5"
        }
    }

    public var supportsVision: Bool { self == .qwen }
    public var requiresReasoningBoundary: Bool { false }
}

public struct StartupConfiguration: Sendable {
    public static let protocolVersion = 1
    public static let maxWireBytes = 64 * 1024
    public static let host = "127.0.0.1"
    public static let port = 0

    public let bearer: String
    public let model: SupportedModel
    public let snapshot: URL

    private struct Wire: Decodable {
        let version: Int
        let bearer: String
        let model: SupportedModel
        let snapshot: String
    }

    public init(data: Data) throws {
        guard !data.isEmpty, data.count <= Self.maxWireBytes,
              let wire = try? JSONDecoder().decode(Wire.self, from: data),
              wire.version == Self.protocolVersion,
              wire.bearer.utf8.count == 64,
              wire.bearer.utf8.allSatisfy({ byte in
                  (48...57).contains(byte) || (97...102).contains(byte)
              }),
              NSString(string: wire.snapshot).isAbsolutePath
        else { throw SidecarError.invalidStartup }

        bearer = wire.bearer
        model = wire.model
        snapshot = URL(fileURLWithPath: wire.snapshot).standardizedFileURL
    }

    public func accepts(_ requested: SupportedModel) -> Bool {
        requested == model
    }
}

public func isAuthorized(_ authorization: String?, bearer: String) -> Bool {
    guard let authorization, authorization.hasPrefix("Bearer ") else { return false }
    let candidate = authorization.dropFirst(7).utf8
    let expected = bearer.utf8
    guard candidate.count == expected.count else { return false }
    return zip(candidate, expected).reduce(0) { $0 | Int($1.0 ^ $1.1) } == 0
}

public struct ChatCompletionRequest: Decodable, Sendable {
    public let model: SupportedModel
    public let messages: [ChatMessage]
    public let stream: Bool
    public let maxTokens: Int?
    public let temperature: Float?

    enum CodingKeys: String, CodingKey {
        case model, messages, stream, temperature
        case maxTokens = "max_tokens"
    }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        model = try values.decode(SupportedModel.self, forKey: .model)
        messages = try values.decode([ChatMessage].self, forKey: .messages)
        stream = try values.decodeIfPresent(Bool.self, forKey: .stream) ?? false
        maxTokens = try values.decodeIfPresent(Int.self, forKey: .maxTokens)
        temperature = try values.decodeIfPresent(Float.self, forKey: .temperature)
        guard !messages.isEmpty,
              maxTokens.map({ (1...32_768).contains($0) }) ?? true,
              temperature.map({ (0...2).contains($0) }) ?? true
        else { throw SidecarError.invalidRequest }
    }

    public func userInput() throws -> UserInput {
        var chat: [Chat.Message] = []
        for message in messages {
            let (text, images) = try message.content.parts()
            if !images.isEmpty && (!model.supportsVision || message.role != .user) {
                throw SidecarError.invalidRequest
            }
            switch message.role {
            case .system: chat.append(.system(text))
            case .user: chat.append(.user(text, images: images))
            case .assistant: chat.append(.assistant(text))
            }
        }
        return UserInput(chat: chat, additionalContext: ["enable_thinking": false])
    }
}

public struct ChatMessage: Decodable, Sendable {
    public enum Role: String, Decodable, Sendable { case system, user, assistant }
    public let role: Role
    public let content: MessageContent
}

public enum MessageContent: Decodable, Sendable {
    case text(String)
    case parts([ContentPart])

    public init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer()
        if let text = try? value.decode(String.self) {
            self = .text(text)
        } else {
            self = .parts(try value.decode([ContentPart].self))
        }
    }

    func parts() throws -> (String, [UserInput.Image]) {
        switch self {
        case .text(let text): return (text, [])
        case .parts(let parts):
            var text = ""
            var images: [UserInput.Image] = []
            for part in parts {
                switch part {
                case .text(let value): text += value
                case .image(let data):
                    guard let image = CIImage(data: data) else { throw SidecarError.invalidRequest }
                    images.append(.ciImage(image))
                }
            }
            guard !text.isEmpty || !images.isEmpty else { throw SidecarError.invalidRequest }
            return (text, images)
        }
    }
}

public enum ContentPart: Decodable, Sendable {
    case text(String)
    case image(Data)

    enum CodingKeys: String, CodingKey { case type, text, imageURL = "image_url" }
    enum ImageKeys: String, CodingKey { case url }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        switch try values.decode(String.self, forKey: .type) {
        case "text":
            self = .text(try values.decode(String.self, forKey: .text))
        case "image_url":
            let image = try values.nestedContainer(keyedBy: ImageKeys.self, forKey: .imageURL)
            let url = try image.decode(String.self, forKey: .url)
            self = .image(try decodeImageDataURL(url))
        default:
            throw SidecarError.invalidRequest
        }
    }
}

func decodeImageDataURL(_ url: String) throws -> Data {
    let maxImageBytes = 16 * 1024 * 1024
    for prefix in ["data:image/jpeg;base64,", "data:image/png;base64,"] where url.hasPrefix(prefix) {
        if let data = Data(base64Encoded: String(url.dropFirst(prefix.count))),
           !data.isEmpty,
           data.count <= maxImageBytes
        {
            return data
        }
    }
    throw SidecarError.invalidRequest
}

public struct ReasoningFilter: Sendable {
    private enum State: Sendable { case prefix, reasoning, answer }
    private var state: State = .prefix
    private var buffer = ""
    private let required: Bool

    public init(required: Bool) { self.required = required }

    public mutating func feed(_ chunk: String, finished: Bool = false) throws -> String {
        buffer += chunk
        var output = ""
        while true {
            switch state {
            case .prefix:
                let trimmed = buffer.drop(while: { $0.isWhitespace })
                if trimmed.hasPrefix("<think>") {
                    buffer = String(trimmed.dropFirst(7))
                    state = .reasoning
                    continue
                }
                if required {
                    if let end = buffer.range(of: "</think>") {
                        buffer = String(buffer[end.upperBound...])
                        state = .answer
                        continue
                    }
                    if finished { throw SidecarError.reasoningBoundaryMissing }
                    return ""
                }
                if "<think>".hasPrefix(String(trimmed)) && !finished { return "" }
                state = .answer
                continue
            case .reasoning:
                guard let end = buffer.range(of: "</think>") else {
                    if finished {
                        if required { throw SidecarError.reasoningBoundaryMissing }
                        buffer = ""
                    }
                    return ""
                }
                buffer = String(buffer[end.upperBound...])
                state = .answer
            case .answer:
                if finished {
                    output += buffer
                    buffer = ""
                } else {
                    let keep = min(6, buffer.count)
                    let split = buffer.index(buffer.endIndex, offsetBy: -keep)
                    output += buffer[..<split]
                    buffer = String(buffer[split...])
                }
                return output
            }
        }
    }
}

public func sseData(_ json: Data) -> String {
    "data: \(String(decoding: json, as: UTF8.self))\n\n"
}
