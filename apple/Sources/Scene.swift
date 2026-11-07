// The view: a spot from its skate file (until GeoGen has meshes for it, GF-REQ-GEOGEN-1), the
// board with its flips, a crude rider (B.17 allows one), and the chase camera sk8 computes (B.16).

import SceneKit
import simd

private func color(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat) -> CGColor { CGColor(red: r, green: g, blue: b, alpha: 1) }

private func material(_ c: CGColor) -> SCNMaterial {
    let m = SCNMaterial()
    m.diffuse.contents = c
    m.lightingModel = .lambert
    return m
}

private func box(_ w: CGFloat, _ h: CGFloat, _ l: CGFloat, _ c: CGColor) -> SCNNode {
    let g = SCNBox(width: w, height: h, length: l, chamferRadius: 0)
    g.materials = [material(c)]
    return SCNNode(geometry: g)
}

/// A rotation whose columns are the board's right, up and forward.
func basis(fwd: SIMD3<Float>, up: SIMD3<Float>) -> simd_quatf {
    let z = simd_normalize(fwd)
    var y = simd_normalize(up)
    let x = simd_normalize(simd_cross(y, z))
    y = simd_cross(z, x)
    return simd_quatf(simd_float3x3(x, y, z))
}

final class SkateScene {
    let scene = SCNScene()
    let camera = SCNNode()
    private let board = SCNNode()
    private let flipper = SCNNode()
    private let rider = SCNNode()
    private let body = SCNNode()
    private var legs: [SCNNode] = []
    private let torso = box(0.22, 0.6, 0.34, color(0.25, 0.49, 0.67))
    private let head = SCNNode(geometry: SCNSphere(radius: 0.11))
    private let arms = box(0.08, 0.08, 1.1, color(0.25, 0.49, 0.67))
    private var spot = SCNNode()

    init() {
        scene.background.contents = color(0.62, 0.71, 0.77)
        scene.fogStartDistance = 60
        scene.fogEndDistance = 220
        scene.fogColor = color(0.62, 0.71, 0.77)
        let amb = SCNNode()
        amb.light = SCNLight()
        amb.light!.type = .ambient
        amb.light!.intensity = 550
        scene.rootNode.addChildNode(amb)
        let sun = SCNNode()
        sun.light = SCNLight()
        sun.light!.type = .directional
        sun.light!.intensity = 900
        sun.light!.castsShadow = true
        sun.light!.shadowRadius = 2
        sun.simdEulerAngles = SIMD3(-1.0, 0.6, 0)
        scene.rootNode.addChildNode(sun)
        camera.camera = SCNCamera()
        camera.camera!.zNear = 0.05
        camera.camera!.zFar = 500
        camera.camera!.fieldOfView = 70
        scene.rootNode.addChildNode(camera)
        buildBoard()
        buildRider()
    }

    private func buildBoard() {
        board.addChildNode(flipper)
        flipper.addChildNode(box(0.21, 0.02, 0.6, color(0.18, 0.23, 0.27)))
        for s: Float in [1, -1] {
            let kick = box(0.2, 0.02, 0.12, s > 0 ? color(0.91, 0.77, 0.28) : color(0.18, 0.23, 0.27))
            kick.simdPosition = SIMD3(0, 0.025, s * 0.35)
            kick.simdEulerAngles = SIMD3(-s * 0.35, 0, 0)
            flipper.addChildNode(kick)
            let truck = box(0.19, 0.03, 0.04, color(0.66, 0.68, 0.7))
            truck.simdPosition = SIMD3(0, -0.03, s * 0.243)
            flipper.addChildNode(truck)
            for x: Float in [0.095, -0.095] {
                let w = SCNNode(geometry: SCNCylinder(radius: 0.031, height: 0.03))
                w.geometry!.materials = [material(color(0.95, 0.94, 0.9))]
                w.simdEulerAngles = SIMD3(0, 0, .pi / 2)
                w.simdPosition = SIMD3(x, -0.044, s * 0.243)
                flipper.addChildNode(w)
            }
        }
        scene.rootNode.addChildNode(board)
    }

    private func buildRider() {
        rider.addChildNode(body)
        for s: Float in [1, -1] {
            let leg = box(0.12, 1, 0.12, color(0.17, 0.17, 0.19))
            leg.simdPosition.z = s * 0.2
            body.addChildNode(leg)
            legs.append(leg)
        }
        head.geometry!.materials = [material(color(0.85, 0.72, 0.6))]
        body.addChildNode(torso)
        body.addChildNode(head)
        body.addChildNode(arms)
        scene.rootNode.addChildNode(rider)
    }

    /// The spot's surfaces and rails.
    func build(_ f: SpotFile) {
        spot.removeFromParentNode()
        spot = SCNNode()
        let colors: [String: CGColor] = [
            "smooth": color(0.73, 0.71, 0.68), "rough": color(0.55, 0.54, 0.52), "slow": color(0.44, 0.56, 0.31),
            "slippery": color(0.66, 0.76, 0.82), "veryslow": color(0.42, 0.35, 0.25), "stairs": color(0.81, 0.79, 0.75),
            "unrideable": color(0.29, 0.31, 0.39),
        ]
        for m in f.meshes {
            var verts: [SCNVector3] = []
            var norms: [SCNVector3] = []
            verts.reserveCapacity(m.triangles.count * 3)
            for t in m.triangles {
                let p = t.map { i -> SIMD3<Float> in let v = m.vertices[Int(i)]; return SIMD3(v[0], v[1], v[2]) }
                let n = simd_normalize(simd_cross(p[1] - p[0], p[2] - p[0]))
                for q in p {
                    verts.append(SCNVector3(q.x, q.y, q.z))
                    norms.append(SCNVector3(n.x, n.y, n.z))
                }
            }
            let idx = (0..<UInt32(verts.count)).map { $0 }
            let g = SCNGeometry(sources: [SCNGeometrySource(vertices: verts), SCNGeometrySource(normals: norms)],
                                elements: [SCNGeometryElement(indices: idx, primitiveType: .triangles)])
            g.materials = [material(colors[m.surface] ?? color(0.6, 0.6, 0.6))]
            spot.addChildNode(SCNNode(geometry: g))
        }
        let metal = material(color(0.85, 0.51, 0.17))
        let edge = material(color(0.94, 0.93, 0.9))
        for r in f.rails {
            var pts = r.points.map { SIMD3<Float>($0[0], $0[1], $0[2]) }
            if r.closed == true, let first = pts.first { pts.append(first) }
            guard pts.count > 1 else { continue }
            for i in 0..<(pts.count - 1) {
                let a = pts[i], b = pts[i + 1]
                let len = simd_length(b - a)
                let thin = r.kind == "thinrail" || r.kind == "coping"
                let c = SCNCylinder(radius: thin ? 0.03 : 0.015, height: CGFloat(len))
                c.materials = [r.material == "metal" ? metal : edge]
                let n = SCNNode(geometry: c)
                n.simdPosition = (a + b) / 2
                n.simdOrientation = simd_quatf(from: SIMD3(0, 1, 0), to: simd_normalize(b - a))
                spot.addChildNode(n)
            }
        }
        scene.rootNode.addChildNode(spot)
    }

    /// Draw the tick between the last two, `alpha` of the way (B.1: render interpolates).
    func update(prev: Sk8Frame, cur: Sk8Frame, alpha: Float) {
        func mix(_ a: SIMD3<Float>, _ b: SIMD3<Float>) -> SIMD3<Float> { a + (b - a) * alpha }
        let pos = mix(prev.position, cur.position)
        let fwd = mix(prev.forward, cur.forward)
        let up = mix(prev.upward, cur.upward)
        board.simdPosition = pos
        board.simdOrientation = basis(fwd: fwd, up: up)
        flipper.simdOrientation = simd_quatf(angle: cur.pitch, axis: SIMD3(1, 0, 0))
            * simd_quatf(angle: cur.shuv_angle, axis: SIMD3(0, 1, 0))
            * simd_quatf(angle: cur.flip_angle, axis: SIMD3(0, 0, 1))
        let state = cur.stateName
        let onBoard = state != "Wipeout" && state != "OffBoard"
        let rup = onBoard ? mix(prev.riderUp, cur.riderUp) : SIMD3<Float>(0, 1, 0)
        rider.simdPosition = onBoard ? pos + up * 0.02 : mix(prev.rider, cur.rider)
        var rf = fwd - rup * simd_dot(fwd, rup)
        if simd_length(rf) < 1e-3 { rf = SIMD3(0, 0, 1) }
        rider.simdOrientation = basis(fwd: rf, up: rup)
        let legH = 0.85 * (1 - 0.38 * cur.crouch)
        for l in legs {
            l.simdScale = SIMD3(1, legH, 1)
            l.simdPosition.y = legH / 2
        }
        torso.simdPosition.y = legH + 0.3
        head.simdPosition.y = legH + 0.72
        arms.simdPosition.y = legH + 0.5
        if state == "Wipeout" {
            // The tumble and its pose are the sim's (Hall of Meat, B.13.1).
            body.simdEulerAngles = SIMD3(cur.body_flip, 0, cur.body_flip * 0.7)
            body.simdPosition.y = -0.6
        } else {
            body.simdPosition.y = 0
            body.simdOrientation = simd_quatf(angle: -cur.lean * (cur.goofy == 1 ? -1 : 1), axis: SIMD3(0, 0, 1))
                * simd_quatf(angle: cur.body_flip, axis: SIMD3(1, 0, 0))
        }
        camera.simdPosition = mix(prev.camera, cur.camera)
        camera.simdLook(at: mix(prev.cameraTarget, cur.cameraTarget), up: SIMD3(0, 1, 0), localFront: SIMD3(0, 0, -1))
        camera.camera!.fieldOfView = CGFloat(cur.cam_fov)
    }
}
