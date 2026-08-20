// swift-tools-version: 6.1

import PackageDescription

let package = Package(
    name: "suflyor-mlx",
    platforms: [.macOS("14.2")],
    products: [
        .executable(name: "suflyor-mlx", targets: ["SuflyorMLX"]),
    ],
    dependencies: [
        .package(url: "https://github.com/ml-explore/mlx-swift-lm.git", exact: "3.31.4"),
        .package(url: "https://github.com/hummingbird-project/hummingbird.git", exact: "2.9.0"),
        .package(url: "https://github.com/huggingface/swift-transformers.git", exact: "1.3.0"),
    ],
    targets: [
        .target(
            name: "SuflyorMLXCore",
            dependencies: [
                .product(name: "Hummingbird", package: "hummingbird"),
                .product(name: "MLXLLM", package: "mlx-swift-lm"),
                .product(name: "MLXVLM", package: "mlx-swift-lm"),
                .product(name: "MLXLMCommon", package: "mlx-swift-lm"),
                .product(name: "MLXHuggingFace", package: "mlx-swift-lm"),
                .product(name: "Tokenizers", package: "swift-transformers"),
            ]
        ),
        .executableTarget(name: "SuflyorMLX", dependencies: ["SuflyorMLXCore"]),
        .testTarget(name: "SuflyorMLXCoreTests", dependencies: ["SuflyorMLXCore"]),
    ]
)
