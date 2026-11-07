// Input (spec C.2): every backend fills the same Sk8Controller, and every sample since the last
// frame goes to the simulation, whose recognizer counts samples (B.4.1).
//
// - Gamepad (iOS, macOS): GCExtendedGamepad, polled each frame (C.2.1).
// - Keyboard (macOS, iPad): GCKeyboard, W/A/S/D on an analog ramp (C.2.3).
// - Mouse as the right stick (macOS, iPad pointer): GCMouse deltas integrated into a stick that
//   springs back when the mouse rests; each delta is a sample (C.2.3).
// - Touch (iOS): two floating sticks, a tap to push, a second finger to brake; coalesced touches
//   are samples (C.2.2). The touch view feeds it (App.swift, TouchPad).

import Foundation
import GameController
import QuartzCore
import simd

enum PadButton {
    static let a: UInt32 = 1 << 0, b: UInt32 = 1 << 1, x: UInt32 = 1 << 2, y: UInt32 = 1 << 3
    static let lb: UInt32 = 1 << 4, rb: UInt32 = 1 << 5, l3: UInt32 = 1 << 6, r3: UInt32 = 1 << 7
    static let dUp: UInt32 = 1 << 8, dDown: UInt32 = 1 << 9, dLeft: UInt32 = 1 << 10, dRight: UInt32 = 1 << 11
}

final class Input {
    private let lock = NSLock()
    private var pending: [Sk8Controller] = []
    private var kb = SIMD2<Float>(0, 0)
    private var mouse = SIMD2<Float>(0, 0)
    private var mouseLast: CFTimeInterval = 0
    /// Touch: the two floating sticks, the trigger buttons, a push pulse and a brake.
    var touchLeft = SIMD2<Float>(0, 0)
    var touchRight = SIMD2<Float>(0, 0)
    var touchLT: Float = 0
    var touchRT: Float = 0
    var touchPushUntil: CFTimeInterval = 0
    var touchBrake = false
    var touchOffUntil: CFTimeInterval = 0
    var mouseSensitivity: Float = 1
    /// ?demo: a scripted ride through the same controller, for self-tests and showing it.
    var demo = false
    private var demoStart: CFTimeInterval = 0
    /// The last sample, for the stick visualiser.
    private(set) var last = Sk8Controller()
    var escapePressed = false
    private var escapeWas = false

    init() {
        NotificationCenter.default.addObserver(forName: .GCMouseDidConnect, object: nil, queue: .main) { [weak self] note in
            (note.object as? GCMouse).map { self?.watch($0) }
        }
        GCMouse.mice().forEach(watch)
    }

    private func watch(_ m: GCMouse) {
        m.mouseInput?.mouseMovedHandler = { [weak self] _, dx, dy in self?.mouseMoved(dx, dy) }
    }

    private func mouseMoved(_ dx: Float, _ dy: Float) {
        lock.lock()
        mouse += SIMD2(dx, dy) / 120 * mouseSensitivity
        let l = simd_length(mouse)
        if l > 1 { mouse /= l }
        mouseLast = CACurrentMediaTime()
        lock.unlock()
        snapshot()
    }

    /// A sample now, from every source (touch and mouse events call this as they come).
    func snapshot() {
        let s = merged(CACurrentMediaTime())
        lock.lock()
        if pending.count < 63 { pending.append(s) }
        lock.unlock()
    }

    /// Everything since the last frame, ending with the state now.
    func drain(dt: Float) -> [Sk8Controller] {
        rampKeys(dt)
        lock.lock()
        if CACurrentMediaTime() - mouseLast > 0.03 { mouse *= max(0, 1 - 12 * dt) }
        lock.unlock()
        snapshot()
        lock.lock()
        let out = pending
        pending.removeAll(keepingCapacity: true)
        lock.unlock()
        last = out.last ?? last
        return out
    }

    private func key(_ k: GCKeyCode) -> Bool {
        GCKeyboard.coalesced?.keyboardInput?.button(forKeyCode: k)?.isPressed ?? false
    }

    private func rampKeys(_ dt: Float) {
        let tx: Float = (key(.keyD) || key(.rightArrow) ? 1 : 0) - (key(.keyA) || key(.leftArrow) ? 1 : 0)
        let ty: Float = (key(.keyW) || key(.upArrow) ? 1 : 0) - (key(.keyS) || key(.downArrow) ? 1 : 0)
        func ramp(_ v: Float, _ t: Float) -> Float {
            if t != 0 && (v == 0 || (v > 0) == (t > 0)) { return max(-1, min(1, v + t * dt / 0.12)) }
            let d = dt / 0.08
            return abs(v) <= d ? (t == 0 ? 0 : t * d) : v - (v > 0 ? d : -d)
        }
        kb = SIMD2(ramp(kb.x, tx), ramp(kb.y, ty))
        let esc = key(.escape) || key(.tab)
        if esc && !escapeWas { escapePressed = true }
        escapeWas = esc
    }

    private func merged(_ now: CFTimeInterval) -> Sk8Controller {
        func pick(_ v: Float...) -> Float { v.max { abs($0) < abs($1) } ?? 0 }
        var c = Sk8Controller()
        var bits: UInt32 = 0
        var lt: Float = 0, rt: Float = 0
        var pad = SIMD4<Float>(0, 0, 0, 0)
        if let g = GCController.current?.extendedGamepad {
            pad = SIMD4(g.leftThumbstick.xAxis.value, g.leftThumbstick.yAxis.value, g.rightThumbstick.xAxis.value, g.rightThumbstick.yAxis.value)
            lt = g.leftTrigger.value
            rt = g.rightTrigger.value
            let map: [(GCControllerButtonInput?, UInt32)] = [
                (g.buttonA, PadButton.a), (g.buttonB, PadButton.b), (g.buttonX, PadButton.x), (g.buttonY, PadButton.y),
                (g.leftShoulder, PadButton.lb), (g.rightShoulder, PadButton.rb), (g.leftThumbstickButton, PadButton.l3),
                (g.rightThumbstickButton, PadButton.r3), (g.dpad.up, PadButton.dUp), (g.dpad.down, PadButton.dDown),
                (g.dpad.left, PadButton.dLeft), (g.dpad.right, PadButton.dRight),
            ]
            for (b, bit) in map where b?.isPressed == true { bits |= bit }
        }
        let keys: [(GCKeyCode, UInt32)] = [(.spacebar, PadButton.a), (.keyC, PadButton.b), (.keyE, PadButton.x), (.keyF, PadButton.y),
                                           (.keyQ, PadButton.lb), (.keyR, PadButton.rb), (.leftShift, PadButton.l3)]
        for (k, bit) in keys where key(k) { bits |= bit }
        if let m = GCMouse.current?.mouseInput {
            if m.leftButton.isPressed { rt = 1 }
            if m.rightButton?.isPressed == true { lt = 1 }
            if m.middleButton?.isPressed == true { bits |= PadButton.r3 }
        }
        lock.lock()
        let ms = mouse
        lock.unlock()
        if now < touchPushUntil { bits |= PadButton.a }
        if touchBrake { bits |= PadButton.b }
        if now < touchOffUntil { bits |= PadButton.y }
        c.lx = pick(kb.x, touchLeft.x, pad.x)
        c.ly = pick(kb.y, touchLeft.y, pad.y)
        c.rx = pick(ms.x, touchRight.x, pad.z)
        c.ry = pick(ms.y, touchRight.y, pad.w)
        c.lt = max(lt, touchLT)
        c.rt = max(rt, touchRT)
        if demo {
            if demoStart == 0 { demoStart = now }
            let t = (now - demoStart).truncatingRemainder(dividingBy: 4)
            if t < 1.5 { bits |= PadButton.a }
            if t > 2.5 && t < 2.6 { c.ry = -1 } else if t >= 2.63 && t < 2.7 { c.ry = 1 }
        }
        c.buttons = bits
        return c
    }
}
