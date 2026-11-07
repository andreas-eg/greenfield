// The mull world. Beargarden runs on it: the world keeps the player, the spot, the board and the
// score. Its web port bridges a WebSocket to the line protocol (a frame is a command line, each
// line the driver writes comes back as a frame). This logs in, rides to the spot, steps on a
// board, says where the board is as sk8 moves it, and tells the world what happened in `skate
// ...` lines (world/support/skate_rules.c), as the lens will (GF-REQ-LENS-2).

import Foundation
import simd

final class WorldLink {
    private var task: URLSessionWebSocketTask?
    private(set) var ready = false
    private var lastWalk: CFTimeInterval = 0
    private var spot = ""
    private var mark = SIMD3<Float>(0, 0, 0)
    private var name = ""
    /// Called on any thread with each line the world says, and with the link's state.
    var onLine: (String) -> Void = { _ in }
    var onStatus: (String) -> Void = { _ in }
    private(set) var lines: [String] = []
    private let lock = NSLock()

    func connect(_ url: URL, spot: String, start: SIMD3<Float>, stance: String, difficulty: String) {
        task?.cancel(with: .goingAway, reason: nil)
        ready = false
        self.spot = spot
        mark = start
        let d = UserDefaults.standard
        name = d.string(forKey: "worldName") ?? "skater\(Int.random(in: 100_000...999_999))"
        let key = d.string(forKey: "worldKey.\(name)") ?? ""
        let t = URLSession.shared.webSocketTask(with: url)
        task = t
        t.resume()
        onStatus("Connecting to \(url.absoluteString)...")
        send(key.isEmpty ? name : "\(name) \(key)")
        receive(t)
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            self?.send("stance \(stance)")
            self?.send("difficulty \(difficulty)")
        }
    }

    func disconnect() {
        task?.cancel(with: .goingAway, reason: nil)
        task = nil
        ready = false
    }

    func send(_ line: String) {
        task?.send(.string(line)) { _ in }
    }

    private func receive(_ t: URLSessionWebSocketTask) {
        t.receive { [weak self] result in
            guard let self, t === self.task else { return }
            switch result {
            case .success(.string(let s)): s.split(separator: "\n").forEach { self.heard(String($0)) }
            case .success(.data(let d)): String(decoding: d, as: UTF8.self).split(separator: "\n").forEach { self.heard(String($0)) }
            case .failure(let e):
                self.ready = false
                self.onStatus("The world is gone: \(e.localizedDescription)")
                return
            @unknown default: break
            }
            self.receive(t)
        }
    }

    private func heard(_ raw: String) {
        let line = raw.trimmingCharacters(in: CharacterSet(charactersIn: "> ").union(.whitespaces))
        guard !line.isEmpty else { return }
        lock.lock()
        lines.append(line)
        if lines.count > 60 { lines.removeFirst() }
        lock.unlock()
        if let r = line.range(of: #"^Your key for '([^']+)' is (\S+)"#, options: .regularExpression) {
            let parts = line[r].split(separator: " ")
            let who = parts[3].trimmingCharacters(in: CharacterSet(charactersIn: "'"))
            UserDefaults.standard.set(who, forKey: "worldName")
            UserDefaults.standard.set(String(parts[5]), forKey: "worldKey.\(who)")
        }
        if line.hasPrefix("Welcome, ") && !ready {
            ready = true
            onStatus("In the world as \(name).")
            // The spot by its name (a ride, the soul's), then to the start, beside the first board.
            send(spot)
            send(String(format: "jump %.2f %.2f", mark.x, mark.z))
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) { [weak self] in self?.send("board") }
        }
        if line.hasPrefix("You step onto a board") || line.hasPrefix("You get into ") {
            onStatus("In the world as \(name), on a board.")
        }
        if line.hasPrefix("Every board here is taken") {
            onStatus("In the world as \(name): every board here is taken.")
        }
        onLine(line)
    }

    /// Each sk8 event the world should hear, as the lens would say it.
    func event(_ e: [String: Any]) {
        guard ready, let type = e["type"] as? String else { return }
        switch type {
        case "banked":
            let pts = Int(((e["points"] as? Double) ?? 0).rounded())
            let mult = (e["multiplier"] as? Double) ?? 1
            let names = (e["names"] as? [String] ?? []).joined(separator: "+")
            send(String(format: "skate banked %d %.1f %@", pts, mult, names))
        case "line_ended": send("skate line \(Int(((e["points"] as? Double) ?? 0).rounded()))")
        case "bail": send("skate bail \(e["reason"] as? String ?? "")")
        case "land": send("skate land \((e["quality"] as? String ?? "").lowercased())")
        case "grind_start": send("skate grind \(e["name"] as? String ?? "")")
        default: break
        }
    }

    /// Where the board is, four times a second.
    func walk(_ f: Sk8Frame, now: CFTimeInterval) {
        guard ready, now - lastWalk >= 0.25 else { return }
        lastWalk = now
        let yaw = atan2(f.fwd.0, f.fwd.2) * 180 / .pi
        send(String(format: "walk %.2f %.2f %.1f", f.pos.0, f.pos.2, yaw))
    }

    var recentLines: [String] {
        lock.lock()
        defer { lock.unlock() }
        return lines
    }
}
