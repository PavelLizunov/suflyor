import QtQuick
import QtQuick.Controls.impl as Impl
import qs.Commons as Commons

// Original product SVGs, tinted by Qt's CPU-capable icon image (no shader).
Impl.ColorImage {
  id: root
  property string name: ""
  property string glyph: ""
  readonly property string resolvedName: name || names[glyph] || ""
  readonly property var names: ({
    "\u{f036c}": "mic", "\uf130": "mic", "\u{f057e}": "system-audio", "\uf028": "voice",
    "\u{f040a}": "play", "\uf04b": "play", "\u{f04db}": "stop", "\uf04d": "stop",
    "\u{f03e4}": "pause", "\uf04c": "pause", "\u{f003c}": "archive", "\uf187": "archive",
    "\u{f0219}": "summary", "\u{f0772}": "refresh", "\uf021": "refresh",
    "\u{f0100}": "camera", "\u{f0493}": "gear", "\uf013": "gear",
    "\u{f02d6}": "sos", "\u{f0709}": "restart", "\u{f05ad}": "x",
    "\u{f0156}": "x", "\uf00d": "x", "\u{f0340}": "unlock", "\u{f033e}": "lock",
    "\u{f03eb}": "pencil", "\uf040": "pencil", "\u{f099b}": "restore",
    "\u{f0293}": "maximize", "\u{f0294}": "minimize", "\uf065": "maximize", "\uf066": "minimize",
    "\uf0c5": "copy", "\uf00c": "check", "\uf141": "more", "\uf142": "grip",
    "\uf019": "update", "\uf1f8": "trash", "\uf005": "star", "\uf062": "arrow-up",
    "\uf060": "seek-back", "\uf061": "seek-forward", "\uf051": "seek-forward", "\uf0f1": "diag"
  })
  width: Commons.Style.space(16)
  height: width
  source: resolvedName ? Qt.resolvedUrl(resolvedName + ".svg") : ""
  color: Commons.Color.foreground
  defaultColor: "#ffffff"
  fillMode: Image.PreserveAspectFit
  sourceSize.width: Math.ceil(width * Screen.devicePixelRatio)
  sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
  asynchronous: true
}
