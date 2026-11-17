// Draws the Hollow Hours icon (a ghost on a dark night tile) into Hollow Hours.iconset.
import AppKit
let out = CommandLine.arguments[1]
try? FileManager.default.createDirectory(atPath: out, withIntermediateDirectories: true)
func draw(_ px: Int) -> Data {
    let s = CGFloat(px)
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let inset = s * 0.1, box = NSRect(x: inset, y: inset * 1.15, width: s - inset * 2, height: s - inset * 2)
    let sh = NSShadow(); sh.shadowBlurRadius = s * 0.025; sh.shadowOffset = NSSize(width: 0, height: -s * 0.012); sh.shadowColor = NSColor.black.withAlphaComponent(0.35); sh.set()
    let tile = NSBezierPath(roundedRect: box, xRadius: box.width * 0.225, yRadius: box.width * 0.225)
    NSGradient(starting: NSColor(red: 0.24, green: 0.07, blue: 0.13, alpha: 1), ending: NSColor(red: 0.05, green: 0.03, blue: 0.07, alpha: 1))!.draw(in: tile, angle: -90)
    NSShadow().set()
    // The ghost: a round head, a body, and a wavy hem.
    let w = box.width * 0.5, cx = box.midX, top = box.maxY - box.height * 0.2, bottom = box.minY + box.height * 0.2
    let g = NSBezierPath()
    g.move(to: NSPoint(x: cx - w / 2, y: bottom))
    g.line(to: NSPoint(x: cx - w / 2, y: top - w / 2))
    g.appendArc(withCenter: NSPoint(x: cx, y: top - w / 2), radius: w / 2, startAngle: 180, endAngle: 0, clockwise: true)
    g.line(to: NSPoint(x: cx + w / 2, y: bottom))
    for i in 0..<4 {
        let x0 = cx + w / 2 - CGFloat(i) * w / 4
        g.curve(to: NSPoint(x: x0 - w / 4, y: bottom), controlPoint1: NSPoint(x: x0 - w / 16, y: bottom + w / 8), controlPoint2: NSPoint(x: x0 - w * 3 / 16, y: bottom - w / 8))
    }
    g.close()
    NSColor(white: 0.95, alpha: 1).setFill(); g.fill()
    NSColor(red: 0.85, green: 0.28, blue: 0.35, alpha: 1).setFill()
    for dx in [-w * 0.18, w * 0.18] { NSBezierPath(ovalIn: NSRect(x: cx + dx - w * 0.07, y: top - w * 0.55, width: w * 0.14, height: w * 0.2)).fill() }
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}
for (n, px) in [("16x16", 16), ("16x16@2x", 32), ("32x32", 32), ("32x32@2x", 64), ("128x128", 128), ("128x128@2x", 256), ("256x256", 256), ("256x256@2x", 512), ("512x512", 512), ("512x512@2x", 1024)] {
    try! draw(px).write(to: URL(fileURLWithPath: "\(out)/icon_\(n).png"))
}
