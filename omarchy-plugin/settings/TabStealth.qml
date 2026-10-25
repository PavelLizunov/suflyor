pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import "controls" as C

// SettingsWindow tab 0: intent and verified capture exclusion differ.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property bool stealth: win ? win.prop("stealth-toggle", false) : false
  readonly property bool effective: win ? win.prop("stealth-effective", false) : false
  readonly property bool macos: {
    if (!service || !service.client) return false
    var revision = service.client.globalsRev
    return (service.client.globals || {})["Platform.is-macos"] === true
  }
  readonly property bool linux: {
    if (!service || !service.client) return true
    var revision = service.client.globalsRev
    var value = (service.client.globals || {})["Platform.is-linux"]
    return value === undefined ? true : value === true
  }
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Window behaviour", []) : ""
    C.DarkCheck {
      text: root.service ? root.service.tr("Always on top", []) : ""
      checked: root.win ? root.win.prop("always-on-top-toggle", true) : true
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "always-on-top-toggle", checked)
        root.service.client.call(root.win.winId, "always-on-top-changed", [checked])
      }
    }
    C.DarkCheck {
      visible: !root.macos
      text: root.service ? root.service.tr("Stealth (hide from screen capture)", []) : ""
      checked: root.stealth
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "stealth-toggle", checked)
        root.service.client.call(root.win.winId, "stealth-changed", [checked])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr(root.macos
        ? "macOS has no public API that can hide the overlay from other screen-sharing apps. Suflyor only excludes its own windows from its own screenshots."
        : root.linux ? "Uses Hyprland capture exclusion. Availability depends on the compositor and the sharing application."
        : "Hides the overlay from screen capture and sharing (Print Screen, Teams, Meet, OBS).", []) : ""
      font.pixelSize: Style.font.body
    }
    C.StatusLine {
      visible: !root.macos && root.stealth
      text: root.service ? root.service.tr(root.effective ? "On, hidden from capture" : "Capture hiding unavailable", []) : ""
      tone: root.effective ? "neutral" : "warning"
      font.pixelSize: Style.font.body
    }
  }
}
