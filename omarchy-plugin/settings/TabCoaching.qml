pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import "controls" as C

// SettingsWindow tab 1: independent debrief and live-speech preferences.
Column {
  id: root
  property var win: null
  property var service: null
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Post-meeting debrief: AI critique of pacing, filler density, key points. Opt-in.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Enable post-meeting debrief", []) : ""
      checked: root.win ? root.win.prop("coaching-debrief", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "coaching-debrief", checked)
        root.service.client.call(root.win.winId, "coaching-debrief-changed", [checked])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Live speech mode phrases the on-screen tile hints as ready-to-read-aloud lines (no filler, confident, short). Independent of the debrief.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Live speech mode — read tiles aloud", []) : ""
      checked: root.win ? root.win.prop("coaching-live-tiles", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "coaching-live-tiles", checked)
        root.service.client.call(root.win.winId, "coaching-live-tiles-changed", [checked])
      }
    }
  }
}
