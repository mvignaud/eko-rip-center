// swift-tools-version: 5.9
// Client Mac natif d'E-KO RIP Center : un Swift Package, sans dépendance externe.
// L'application (.app) est assemblée par construire.sh.
import PackageDescription

let package = Package(
    name: "EKORIPCenter",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "EKORIPCenter",
            path: "Sources/EKORIPCenter"
        ),
        .testTarget(
            name: "EKORIPCenterTests",
            dependencies: ["EKORIPCenter"],
            path: "Tests/EKORIPCenterTests"
        ),
    ]
)
