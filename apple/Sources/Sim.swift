// The sk8 simulation (beargarden/sk8, linked as libsk8.a) seen from Swift, and the spot files
// it rides (world/charts/<spot>.skate.json, bundled as "spots").

import Foundation
import simd

/// A spot's file: the triangles by surface category, the rails, the marks (sk8::world::SpotFile).
struct SpotFile: Decodable {
    struct Mesh: Decodable {
        let surface: String
        let vertices: [[Float]]
        let triangles: [[UInt32]]
    }
    struct Rail: Decodable {
        let kind: String
        let material: String
        let closed: Bool?
        let points: [[Float]]
    }
    struct Mark: Decodable {
        let name: String
        let pos: [Float]
        let heading: Float?
    }
    let name: String
    let title: String?
    let spawn: [Float]
    let meshes: [Mesh]
    let rails: [Rail]
    let marks: [Mark]

    func mark(_ name: String) -> SIMD3<Float> {
        let p = marks.first { $0.name == name }?.pos ?? spawn
        return SIMD3(p[0], p[1], p[2])
    }
}

enum Spots {
    static let all: [(id: String, title: String)] = [
        ("plattan", "Plattan, Sergels torg"),
        ("ralis", "Rålis, Rålambshovsparken"),
        ("fryshuset", "Fryshuset"),
        ("ovningsplanen", "Övningsplanen"),
    ]
    static func data(_ id: String) -> Data? {
        guard let url = Bundle.main.url(forResource: id + ".skate", withExtension: "json", subdirectory: "spots") else { return nil }
        return try? Data(contentsOf: url)
    }
}

extension Sk8Frame {
    var position: SIMD3<Float> { SIMD3(pos.0, pos.1, pos.2) }
    var forward: SIMD3<Float> { SIMD3(fwd.0, fwd.1, fwd.2) }
    var upward: SIMD3<Float> { SIMD3(up.0, up.1, up.2) }
    var rider: SIMD3<Float> { SIMD3(rider_pos.0, rider_pos.1, rider_pos.2) }
    var riderUp: SIMD3<Float> { SIMD3(rider_up.0, rider_up.1, rider_up.2) }
    var camera: SIMD3<Float> { SIMD3(cam_pos.0, cam_pos.1, cam_pos.2) }
    var cameraTarget: SIMD3<Float> { SIMD3(cam_target.0, cam_target.1, cam_target.2) }
    /// sk8::sim::State, by its number.
    var stateName: String {
        ["Ground", "Powerslide", "Revert", "Air", "KnownAir", "Grind", "Wipeout", "OffBoard", "WallRide", "Handplant"][min(Int(state), 9)]
    }
}

/// One simulation of one spot.
final class Sim {
    private let h: OpaquePointer
    let spot: SpotFile

    init?(spot id: String, difficulty: UInt32, goofy: Bool) {
        guard let data = Spots.data(id), let file = try? JSONDecoder().decode(SpotFile.self, from: data) else { return nil }
        let handle = data.withUnsafeBytes { raw in
            sk8_new(raw.bindMemory(to: UInt8.self).baseAddress, raw.count, difficulty)
        }
        guard let handle else { return nil }
        h = handle
        spot = file
        sk8_set_stance(h, goofy ? 1 : 0)
    }

    deinit { sk8_free(h) }

    /// The recognizer's windows are in samples: say how many come a second (B.4.1).
    func setSampleRate(_ hz: Float) { sk8_set_sample_rate(h, hz) }

    @discardableResult
    func advance(_ dt: Float, _ samples: [Sk8Controller]) -> UInt32 {
        samples.withUnsafeBufferPointer { sk8_advance(h, dt, $0.baseAddress, $0.count) }
    }

    var frame: Sk8Frame { sk8_frame(h).pointee }
    var previous: Sk8Frame { sk8_prev_frame(h).pointee }
    var alpha: Float { min(max(sk8_alpha(h), 0), 1) }

    /// Events since the last call, each a JSON object (sk8::sim::Event).
    func events() -> [[String: Any]] {
        let n = sk8_events(h, nil, 0)
        guard n > 0 else { return [] }
        var buf = [UInt8](repeating: 0, count: n)
        _ = buf.withUnsafeMutableBufferPointer { sk8_events(h, $0.baseAddress, n) }
        return String(decoding: buf, as: UTF8.self).split(separator: "\n").compactMap {
            try? JSONSerialization.jsonObject(with: Data($0.utf8)) as? [String: Any]
        }
    }
}
