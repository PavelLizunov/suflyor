pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import "controls" as C

// SettingsWindow tab 7: detector preferences and Omarchy tile inactivity.
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
      text: root.service ? root.service.tr("Trigger words in the transcript create answer tiles.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Enable auto-tile detector", []) : ""
      checked: root.win ? root.win.prop("auto-tiles-enabled", true) : true
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "auto-tiles-enabled", checked)
        root.service.client.call(root.win.winId, "auto-tiles-enabled-changed", [checked])
      }
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Fast answers for calls and interviews", []) : ""
      checked: root.win ? root.win.prop("fast-answers", true) : true
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "fast-answers", checked)
        root.service.client.call(root.win.winId, "fast-answers-changed", [checked])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("A one-line answer, then two or three relevant points; terms under the answer open their definitions.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Listening mode — don't show tiles", []) : ""
      checked: root.win ? root.win.prop("suppress-tiles", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "suppress-tiles", checked)
        root.service.client.call(root.win.winId, "suppress-tiles-changed", [checked])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Only auto-tiles are muted; manual asks still work.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Close inactive tiles after (seconds; 0 = never)", []) : ""
      C.SettingsSlider {
        minimum: 0
        maximum: 600
        step: 1
        value: root.win ? root.win.prop("tile-idle-seconds", 30) : 30
        onChanged: function(value) {
          if (root.win && root.service) root.service.client.set(root.win.winId, "tile-idle-seconds", Math.round(value))
        }
        onReleased: function(value) {
          if (root.win && root.service) root.service.client.call(root.win.winId, "tile-idle-seconds-changed", [Math.round(value)])
        }
      }
      C.SettingsCaption {
        text: String(root.win ? root.win.prop("tile-idle-seconds", 30) : 30)
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Omarchy only. Interaction restarts the timer. Answers, recording, playback and unsent edits keep the tile open.", []) : ""
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Separate words with spaces; questions also trigger.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsTextEdit {
      minimumHeight: Style.space(150)
      text: root.win ? root.win.prop("trigger-keywords-input", "") : ""
      placeholderText: root.service ? root.service.tr("e.g. kubernetes docker postgres", []) : ""
      onEdited: function(text) {
        if (root.win && root.service && text !== root.win.prop("trigger-keywords-input", ""))
          root.service.client.set(root.win.winId, "trigger-keywords-input", text)
      }
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Save words", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "trigger-keywords-save", [root.win.prop("trigger-keywords-input", "")])
    }
  }
}
