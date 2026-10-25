pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons
import "controls" as C

// SettingsWindow tab 13. Hotkey detail precedence matches Slint: registering, conflict, list.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property int hotkeysLevel: win ? win.prop("diag-hotkeys-level", -1) : -1
  readonly property string hotkeysFailed: win ? win.prop("diag-hotkeys-failed", "") : ""
  readonly property string hotkeysDetail: win ? win.prop("diag-hotkeys-detail", "") : ""
  readonly property string armed: win ? win.prop("db-clear-armed", "") : ""
  readonly property var readiness: [
    { label: "AI (answers)", level: win ? win.prop("diag-ai-level", -1) : -1, detail: win ? win.prop("diag-ai-detail", "") : "" },
    { label: "Speech recognition (STT)", level: win ? win.prop("diag-stt-level", -1) : -1, detail: win ? win.prop("diag-stt-detail", "") : "" },
    { label: "Screenshots (F8)", level: win ? win.prop("diag-vision-level", -1) : -1, detail: (win ? win.prop("diag-vision-detail", "") : "") || (service ? service.tr("off", []) : "") },
    { label: "Microphone", level: win ? win.prop("diag-mic-level", -1) : -1, detail: (win ? win.prop("diag-mic-detail", "") : "") || (service ? service.tr("default", []) : "") },
    { label: "System audio", level: win ? win.prop("diag-sys-level", -1) : -1, detail: (win ? win.prop("diag-sys-detail", "") : "") || (service ? service.tr("default", []) : "") },
    { label: "Global hotkeys", level: hotkeysLevel, detail: hotkeysLevel === 3 ? (service ? service.tr("registering…", []) : "") : hotkeysFailed !== "" ? (service ? service.tr("conflict: {}", [hotkeysFailed]) : "") : hotkeysDetail !== "" ? hotkeysDetail : (service ? service.tr("hotkey system unavailable", []) : "") },
    { label: "Stealth", level: 3, detail: service ? service.tr((win ? win.prop("diag-stealth-on", false) : false) ? "on — verify with a screen-share preview" : "off", []) : "" }
  ]
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Session readiness", []) : ""
    C.SettingsCaption {
      text: root.win ? root.win.prop("diag-summary", "") : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsSeparator {}
    Repeater {
      model: root.readiness
      delegate: Item {
        id: row
        required property var modelData
        width: parent.width
        implicitHeight: Math.max(label.implicitHeight, state.implicitHeight, detail.implicitHeight) + Style.space(8)
        height: implicitHeight
        C.SettingsCaption {
          id: label
          x: Style.space(4)
          y: Style.space(4)
          width: Math.min(Style.space(150), row.width * 0.3)
          text: root.service ? root.service.tr(row.modelData.label, []) : ""
          color: Commons.Color.foreground
          font.pixelSize: Style.font.body
          font.weight: Font.DemiBold
        }
        C.SettingsCaption {
          id: state
          x: label.x + label.width + Style.space(10)
          y: Style.space(4)
          width: Math.min(Style.space(100), row.width * 0.23)
          text: root.service ? root.service.tr(row.modelData.level === 0 ? "Ready"
            : row.modelData.level === -1 ? "checking…"
            : row.modelData.level === 1 ? "No signal"
            : row.modelData.level === 2 ? "Not configured"
            : row.modelData.level === 4 ? "Error" : "—", []) : ""
          color: row.modelData.level === 0 ? Commons.Color.accent
            : row.modelData.level === 1 || row.modelData.level === 2 || row.modelData.level === 4 ? Commons.Color.urgent : Commons.Color.muted
          font.pixelSize: Style.font.body
          font.bold: true
        }
        C.SettingsCaption {
          id: detail
          x: state.x + state.width + Style.space(10)
          y: Style.space(4)
          width: Math.max(0, row.width - x - Style.space(4))
          text: row.modelData.detail
          font.pixelSize: Style.font.body
        }
      }
    }
    C.SettingsButtonsRow {
      C.SettingsButton {
        text: root.service ? root.service.tr("Check all", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "diagnostics-check-all-clicked", [])
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Copy report", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "diagnostics-copy-report-clicked", [])
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Collect logs", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "diagnostics-collect-logs-clicked", [])
      }
    }
    C.StatusLine {
      visible: root.win ? root.win.prop("diag-copied", false) : false
      text: root.service ? root.service.tr("copied", []) : ""
      tone: "good"
      font.pixelSize: Style.font.body
      font.bold: true
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("“Check all” pings AI + STT, records the mic, and plays a test tone to verify system-audio capture. Speak during the check to see the mic level — you don't need to play anything yourself.", []) : ""
    }
    C.StatusLine {
      text: root.win ? root.win.prop("diag-logs-path", "") : ""
      tone: "good"
      font.pixelSize: Style.font.body
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Database", []) : ""
    C.SettingsButton {
      text: root.service ? root.service.tr("Check & repair database", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "db-repair-clicked", [])
    }
    C.SettingsDangerButton {
      text: root.service ? root.service.tr(root.armed === "queue" ? "Tap again — clear the suggestions queue" : "Clear suggestions queue", []) : ""
      onClicked: {
        if (!root.win || !root.service) return
        if (root.armed === "queue") {
          root.service.client.set(root.win.winId, "db-clear-armed", "")
          root.service.client.call(root.win.winId, "db-clear-queue-clicked", [])
        } else {
          root.service.client.set(root.win.winId, "db-clear-armed", "queue")
        }
      }
    }
    C.SettingsDangerButton {
      text: root.service ? root.service.tr(root.armed === "memory" ? "Tap again — clear approved memory" : "Clear approved memory", []) : ""
      onClicked: {
        if (!root.win || !root.service) return
        if (root.armed === "memory") {
          root.service.client.set(root.win.winId, "db-clear-armed", "")
          root.service.client.call(root.win.winId, "db-clear-memory-clicked", [])
        } else {
          root.service.client.set(root.win.winId, "db-clear-armed", "memory")
        }
      }
    }
    C.StatusLine {
      text: root.win ? root.win.prop("db-repair-status", "") : ""
      font.pixelSize: Style.font.body
    }
  }
}
