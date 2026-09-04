import Darwin
import Foundation
import SuflyorMLXCore

#if !arch(arm64)
#error("suflyor-mlx is an arm64-only sidecar")
#endif

@main
enum Main {
    static func main() async {
        do {
            guard CommandLine.arguments.count == 1 else { throw SidecarError.invalidStartup }
            let startup = try StartupConfiguration(data: readStartupLine())
            try await SidecarServer(startup: startup).run()
        } catch {
            FileHandle.standardError.write(Data("suflyor-mlx failed\n".utf8))
            Darwin.exit(1)
        }
    }

    private static func readStartupLine() throws -> Data {
        var line = Data()
        line.reserveCapacity(512)
        var buffer = [UInt8](repeating: 0, count: 4096)
        while line.count <= StartupConfiguration.maxWireBytes {
            let maxChunk = min(buffer.count, StartupConfiguration.maxWireBytes - line.count)
            let bytesRead = Darwin.read(STDIN_FILENO, &buffer, maxChunk)
            guard bytesRead > 0 else {
                throw SidecarError.invalidStartup
            }
            let chunk = Data(buffer[0..<bytesRead])
            if let newlineIndex = chunk.firstIndex(of: 0x0A) {
                line.append(chunk[chunk.startIndex..<newlineIndex])
                return line
            }
            line.append(chunk)
        }
        throw SidecarError.invalidStartup
    }
}
