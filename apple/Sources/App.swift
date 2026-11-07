// Beargarden on iOS and macOS, native: the sk8 simulation linked in (libsk8.a), stepped from
// SceneKit's display-linked render loop at the display's rate with sk8's fixed 1/60 s tick and
// interpolation inside (B.1, C.2.5); GameController for pads, keyboards and mice; touch sticks
// on iOS; the mull world over its WebSocket bridge.
//
// Launch arguments (or the same keys in the defaults):
//   -world ws://host:4062/ws   join that world at once     -spot ralis      which spot
//   -demo YES                  a scripted ride              -selftest 12     ride 12 s, render
//   -snapshot /path/shot.png   offscreen (works with no screen), print a summary, quit

import SwiftUI
import SceneKit
import GameController
#if os(iOS)
import UIKit
#else
import AppKit
#endif

final class HUD: ObservableObject {
    @Published var score = 0
    @Published var speed: Float = 0
    @Published var names = ""
    @Published var points = ""
    @Published var flash = ""
    @Published var flashBad = false
    @Published var status = ""
    @Published var lineTimer: Float = 0
    @Published var state = ""
    @Published var fakie = false
    @Published var menu = true
    @Published var world = ""
}

final class Game: NSObject, SCNSceneRendererDelegate {
    let view = SkateScene()
    let input = Input()
    let world = WorldLink()
    let hud = HUD()
    private var sim: Sim?
    private let lock = NSLock()
    private var last: CFTimeInterval = 0
    private var hudLast: CFTimeInterval = 0
    private var flashUntil: CFTimeInterval = 0
    private var namesUntil: CFTimeInterval = 0
    private var sampleCount = 0
    private var rateClock: Float = 0
    private var rate: Float = 60
    var selfTest = false
    private(set) var states: [String: Int] = [:]
    private(set) var tricks: [String] = []
    private(set) var lastFrame = Sk8Frame()

    override init() {
        super.init()
        world.onStatus = { [weak self] s in DispatchQueue.main.async { self?.hud.world = s } }
        world.onLine = { [weak self] line in
            if line.hasPrefix("Time!") || line.hasPrefix("Two minutes") || line.hasPrefix("Step on") || line.hasPrefix("A session") || line.hasPrefix("Sessions are") {
                self?.flash(line, bad: false)
            }
        }
    }

    func start(spot: String, difficulty: Int, goofy: Bool, worldURL: String) {
        lock.lock()
        defer { lock.unlock() }
        guard let s = Sim(spot: spot, difficulty: UInt32(difficulty), goofy: goofy) else {
            hud.status = "Could not load \(spot)"
            return
        }
        sim = s
        last = 0
        view.build(s.spot)
        if let url = URL(string: worldURL), !worldURL.isEmpty {
            world.connect(url, spot: spot, start: s.spot.mark("start"), stance: goofy ? "goofy" : "regular",
                          difficulty: ["easy", "normal", "hardcore"][max(0, min(2, difficulty))])
        } else {
            world.disconnect()
            hud.world = "Skating alone: nothing is kept."
        }
    }

    func renderer(_ renderer: SCNSceneRenderer, updateAtTime time: TimeInterval) {
        if !selfTest { tick(CACurrentMediaTime()) }
    }

    func tick(_ now: CFTimeInterval) {
        lock.lock()
        defer { lock.unlock() }
        guard let sim else { return }
        if last == 0 { last = now; return }
        let dt = Float(min(0.1, now - last))
        last = now
        let samples = input.drain(dt: dt)
        sampleCount += samples.count
        rateClock += dt
        if rateClock >= 1 {
            let r: Float = Float(sampleCount) / rateClock > 90 ? 120 : 60
            if r != rate { rate = r; sim.setSampleRate(r) }
            sampleCount = 0
            rateClock = 0
        }
        sim.advance(dt, samples)
        let cur = sim.frame
        lastFrame = cur
        states[cur.stateName, default: 0] += 1
        for e in sim.events() {
            world.event(e)
            onEvent(e)
        }
        world.walk(cur, now: now)
        view.update(prev: sim.previous, cur: cur, alpha: sim.alpha)
        if input.escapePressed {
            input.escapePressed = false
            DispatchQueue.main.async { self.hud.menu.toggle() }
        }
        if now - hudLast > 0.1 {
            hudLast = now
            let names = now < namesUntil || cur.sequence > 0
            DispatchQueue.main.async {
                self.hud.score = Int((cur.score + cur.line).rounded())
                self.hud.speed = cur.speed
                self.hud.fakie = cur.fakie == 1
                self.hud.state = cur.stateName
                self.hud.lineTimer = cur.line_timer
                if !names { self.hud.names = ""; if cur.line == 0 { self.hud.points = "" } }
                if cur.sequence > 0 { self.hud.points = "\(Int(cur.sequence))" + (cur.multiplier > 1 ? "  x\(cur.multiplier)" : "") }
                if now > self.flashUntil { self.hud.flash = "" }
            }
        }
    }

    private func onEvent(_ e: [String: Any]) {
        let now = CACurrentMediaTime()
        switch e["type"] as? String {
        case "pop": haptic(0.5)
        case "trick", "grind_start":
            let n = e["name"] as? String ?? ""
            tricks.append(n)
            namesUntil = now + 2.5
            DispatchQueue.main.async { self.hud.names = n }
        case "land":
            haptic(0.8)
            if let q = e["quality"] as? String, q != "Clean" { flash(q.uppercased(), bad: false) }
        case "banked":
            namesUntil = now + 2.5
            let names = (e["names"] as? [String] ?? []).joined(separator: " + ")
            let pts = Int((e["points"] as? Double ?? 0).rounded())
            let m = e["multiplier"] as? Double ?? 1
            DispatchQueue.main.async { self.hud.names = names; self.hud.points = "+\(pts)" + (m > 1 ? "  x\(m)" : "") }
        case "line_ended":
            let p = Int((e["points"] as? Double ?? 0).rounded())
            if p > 0 { flash("LINE +\(p)", bad: false) }
        case "bail":
            haptic(1)
            flash("BAIL: \(e["reason"] as? String ?? "")", bad: true)
        case "run_out": flash("RUN OUT", bad: true)
        default: break
        }
    }

    private func flash(_ text: String, bad: Bool) {
        flashUntil = CACurrentMediaTime() + 1.2
        DispatchQueue.main.async { self.hud.flash = text; self.hud.flashBad = bad }
    }

    private func haptic(_ strength: CGFloat) {
#if os(iOS)
        DispatchQueue.main.async { UIImpactFeedbackGenerator(style: .medium).impactOccurred(intensity: strength) }
#endif
    }

    /// Render what the camera sees now, offscreen: no window or screen needed.
    func snapshot(to path: String) -> Bool {
        let r = SCNRenderer(device: MTLCreateSystemDefaultDevice(), options: nil)
        r.scene = view.scene
        r.pointOfView = view.camera
        let image = r.snapshot(atTime: 0, with: CGSize(width: 1280, height: 720), antialiasingMode: .multisampling4X)
#if os(iOS)
        guard let data = image.pngData() else { return false }
#else
        guard let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
              let data = rep.representation(using: .png, properties: [:]) else { return false }
#endif
        return (try? data.write(to: URL(fileURLWithPath: path))) != nil
    }
}

// ---- the views ------------------------------------------------------------------------------

#if os(iOS)
struct SceneViewRep: UIViewRepresentable {
    let game: Game
    func makeUIView(context: Context) -> SCNView { makeSCNView(game) }
    func updateUIView(_ v: SCNView, context: Context) {}
}

/// Two floating sticks (C.2.2): where a thumb lands is the stick's centre, 60 pt is full travel.
final class TouchPadView: UIView {
    weak var input: Input?
    private var left: (UITouch, CGPoint, CFTimeInterval, CGFloat)?
    private var right: (UITouch, CGPoint)?
    private var extraLeft = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = .clear
    }
    required init?(coder: NSCoder) { fatalError() }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        for t in touches {
            let p = t.location(in: self)
            if p.x < bounds.midX {
                if left == nil { left = (t, p, CACurrentMediaTime(), 0) } else { extraLeft += 1; input?.touchBrake = true }
            } else if right == nil {
                right = (t, p)
            }
        }
    }

    private func stick(_ from: CGPoint, _ to: CGPoint) -> SIMD2<Float> {
        var v = SIMD2<Float>(Float(to.x - from.x), Float(from.y - to.y)) / 60
        let l = simd_length(v)
        if l > 1 { v /= l }
        return v
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        for t in touches {
            let all = event?.coalescedTouches(for: t) ?? [t]
            for c in all {
                let p = c.location(in: self)
                if let l = left, l.0 === t {
                    input?.touchLeft = stick(l.1, p)
                    left?.3 = max(l.3, hypot(p.x - l.1.x, p.y - l.1.y))
                } else if let r = right, r.0 === t {
                    input?.touchRight = stick(r.1, p)
                }
                input?.snapshot()
            }
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        for t in touches {
            if let l = left, l.0 === t {
                if CACurrentMediaTime() - l.2 < 0.15 && l.3 < 8 { input?.touchPushUntil = CACurrentMediaTime() + 0.06 }
                left = nil
                input?.touchLeft = .zero
            } else if let r = right, r.0 === t {
                right = nil
                input?.touchRight = .zero
            } else if extraLeft > 0 {
                extraLeft -= 1
                if extraLeft == 0 { input?.touchBrake = false }
            }
        }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { touchesEnded(touches, with: event) }
}

struct TouchPad: UIViewRepresentable {
    let input: Input
    func makeUIView(context: Context) -> TouchPadView {
        let v = TouchPadView(frame: .zero)
        v.input = input
        return v
    }
    func updateUIView(_ v: TouchPadView, context: Context) {}
}
#else
struct SceneViewRep: NSViewRepresentable {
    let game: Game
    func makeNSView(context: Context) -> SCNView { makeSCNView(game) }
    func updateNSView(_ v: SCNView, context: Context) {}
}
#endif

func makeSCNView(_ game: Game) -> SCNView {
    let v = SCNView(frame: .zero, options: nil)
    v.scene = game.view.scene
    v.pointOfView = game.view.camera
    v.delegate = game
    v.isPlaying = true
    v.rendersContinuously = true
    v.preferredFramesPerSecond = 120
    v.antialiasingMode = .multisampling4X
    return v
}

struct GameScreen: View {
    let game: Game
    @ObservedObject var hud: HUD
    @AppStorage("spot") private var spot = "plattan"
    @AppStorage("difficulty") private var difficulty = 1
    @AppStorage("goofy") private var goofy = false
    @AppStorage("worldURL") private var worldURL = ""

    var body: some View {
        ZStack {
            SceneViewRep(game: game).ignoresSafeArea()
#if os(iOS)
            TouchPad(input: game.input).ignoresSafeArea()
#endif
            hudView
            if hud.menu { menu }
        }
        .onChange(of: hud.menu) { _, open in capture(!open) }
    }

    private var hudView: some View {
        VStack {
            HStack(alignment: .top) {
                Text(hud.world).font(.caption).foregroundStyle(.white.opacity(0.7))
                Spacer()
                VStack(alignment: .center, spacing: 2) {
                    Text(hud.names).font(.headline)
                    Text(hud.points).font(.subheadline.monospacedDigit()).foregroundStyle(.yellow)
                    Capsule().fill(.yellow).frame(width: CGFloat(240 * min(1, max(0, hud.lineTimer / 5))), height: 4)
                }
                Spacer()
                VStack(alignment: .trailing) {
                    Text("\(hud.score)").font(.system(size: 30, weight: .bold).monospacedDigit())
                    Text(String(format: "%.0f km/h%@", hud.speed * 3.6, hud.fakie ? "  fakie" : "")).font(.caption)
                }
            }
            Spacer()
            Text(hud.flash).font(.title.bold()).foregroundStyle(hud.flashBad ? .red : .white)
            Spacer()
            HStack {
                Text(hud.state).font(.caption).foregroundStyle(.white.opacity(0.7))
                Spacer()
#if os(iOS)
                Button("MENU") { hud.menu = true }.buttonStyle(.bordered)
#endif
            }
        }
        .foregroundStyle(.white)
        .shadow(radius: 2)
        .padding()
        .allowsHitTesting(false)
    }

    private var menu: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Beargarden sk8r").font(.title2.bold())
            Text("Stockholm skate, on the sk8 simulation").font(.caption).foregroundStyle(.secondary)
            Picker("Spot", selection: $spot) { ForEach(Spots.all, id: \.id) { Text($0.title).tag($0.id) } }
            Picker("Difficulty", selection: $difficulty) { Text("Easy").tag(0); Text("Normal").tag(1); Text("Hardcore").tag(2) }
            Toggle("Goofy (right foot forward)", isOn: $goofy)
            TextField("World: ws://host:4062/ws, empty to skate alone", text: $worldURL)
                .textFieldStyle(.roundedBorder)
#if os(iOS)
                .textInputAutocapitalization(.never).autocorrectionDisabled()
#endif
            HStack {
                Button("Skate") {
                    game.start(spot: spot, difficulty: difficulty, goofy: goofy, worldURL: worldURL)
                    hud.menu = false
                }.keyboardShortcut(.defaultAction)
                Button("Start a session") { game.world.send("session"); hud.menu = false }.disabled(!game.world.ready)
            }
            Text("Pad: left stick body, right stick board (pull back, flick up to ollie). Keys: WASD, Space push, mouse is the board, Esc menu. Touch: left half body, right half board, tap left to push.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: 380)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    /// The mouse is the right stick on the Mac: captured while skating, freed in the menu.
    private func capture(_ on: Bool) {
#if os(macOS)
        CGAssociateMouseAndMouseCursorPosition(on ? 0 : 1)
        if on { NSCursor.hide() } else { NSCursor.unhide() }
#endif
    }
}

@main
struct BeargardenApp: App {
    private let game = Game()

    init() {
        let d = UserDefaults.standard
        let g = game
        g.input.demo = d.bool(forKey: "demo")
        let world = d.string(forKey: "world")
        let spot = d.string(forKey: "spot") ?? "plattan"
        if world != nil || g.input.demo || d.double(forKey: "selftest") > 0 {
            g.start(spot: spot, difficulty: d.object(forKey: "difficulty") as? Int ?? 1, goofy: d.bool(forKey: "goofy"), worldURL: world ?? "")
            g.hud.menu = false
        }
        let seconds = d.double(forKey: "selftest")
        if seconds > 0 { BeargardenApp.runSelfTest(g, seconds: seconds, snapshot: d.string(forKey: "snapshot")) }
    }

    /// Ride for `seconds` on a timer (no window needs to be on a screen), render offscreen,
    /// print what happened, quit.
    static func runSelfTest(_ g: Game, seconds: Double, snapshot: String?) {
        g.selfTest = true
        let t0 = CACurrentMediaTime()
        Timer.scheduledTimer(withTimeInterval: 1.0 / 60.0, repeats: true) { timer in
            let now = CACurrentMediaTime()
            g.tick(now)
            guard now - t0 >= seconds else { return }
            timer.invalidate()
            let shot = snapshot.map { g.snapshot(to: $0) } ?? false
            let f = g.lastFrame
            let summary: [String: Any] = [
                "states": g.states, "tricks": Array(g.tricks.suffix(8)), "score": Int(f.score + f.line),
                "position": [f.pos.0, f.pos.1, f.pos.2], "world": g.hud.world, "world_said": Array(g.world.recentLines.suffix(6)),
                "snapshot": shot,
            ]
            if let data = try? JSONSerialization.data(withJSONObject: summary), let s = String(data: data, encoding: .utf8) {
                print("BEARGARDEN-SELFTEST " + s)
            }
            fflush(stdout)
            UserDefaults.standard.synchronize()
            exit(0)
        }
    }

    var body: some Scene {
        WindowGroup {
            GameScreen(game: game, hud: game.hud)
#if os(iOS)
                .statusBarHidden(true)
                .persistentSystemOverlays(.hidden)
#endif
        }
#if os(macOS)
        .defaultSize(width: 1280, height: 720)
#endif
    }
}
