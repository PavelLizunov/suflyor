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
        while line.count <= StartupConfiguration.maxWireBytes {
            let remaining = StartupConfiguration.maxWireBytes - line.count
            let toRead = min(4096, max(1, remaining))
            guard let chunk = try FileHandle.standardInput.read(upToCount: toRead), !chunk.isEmpty else {
                throw SidecarError.invalidStartup
            }
            if let newlineIndex = chunk.firstIndex(of: 0x0A) {
                line.append(chunk[chunk.startIndex..<newlineIndex])
                return line
            }
            line.append(chunk)
        }
        throw SidecarError.invalidStartup
    }
}
