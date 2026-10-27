pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import Quickshell.Wayland
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui
import "bar" as Bar
import "icons" as Icons

// The bar widget supplies anchorItem, hostWidget, service and the inherited bar.
Ui.Panel {
  id: root

  moduleName: "io.github.pavellizunov.suflyor"
  ipcTarget: moduleName
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null
  property var service: null
  readonly property var win: service ? service.client.first("OverlayBarWindow") : null
  readonly property var lockWin: service ? service.client.first("LockModeMenuWindow") : null
  readonly property bool ready: !!service && service.running && !!win
  readonly property bool bootstrap: p("bootstrap-mode", false)
  readonly property bool compact: p("compact-bar", false)
  readonly property bool sessionActive: p("timer-active", false)
  readonly property bool recording: p("mic-recording", false) || p("sys-recording", false)
  readonly property bool micCapturing: p("mic-recording", false)
    || (sessionActive && !p("session-paused", false) && !p("mic-muted", false))
  readonly property bool confirming: p("quit-armed", false) || p("restart-armed", false)
  readonly property bool stealth: p("stealth-active", false)
  readonly property bool suppress: p("suppress-tiles", false)
  readonly property bool deep: p("deep-lock", false)
  // This is a computed Slint `out` binding, not daemon-owned state. The remote
  // generator stores its default, so do not trust its lock-icon-unlocked snapshot.
  readonly property bool lockIconUnlocked: !suppress && !deep
  readonly property bool remoteLockVisible: !!lockWin && lockWin.visible
  readonly property bool lockManaged: remoteLockVisible ? lockWin.prop("managed", false) : p("lock-menu-managed", false)
  readonly property int lockMode: remoteLockVisible ? lockWin.prop("mode", 0) : (deep ? 2 : (suppress ? 1 : 0))
  property bool localLockOpen: false
  // Offscreen previews draw this subtree on their own canvas (tools/remote-ui/preview).
  readonly property Item contentRoot: keyCatcher
  readonly property int scrollGutter: Style.space(14)
  readonly property bool lockMenuOpen: localLockOpen || remoteLockVisible

  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function tr(msgid, args) { return root.service ? root.service.tr(msgid, args || []) : msgid }
  function fire(name, args) {
    if (!root.ready) return
    // Release the popout's keyboard/pointer ownership before another surface opens.
    switch (name) {
    case "open-settings-clicked":
    case "archive-clicked":
    case "help-clicked":
    case "text-ask-clicked":
    case "spawn-tile-clicked":
    case "restore-tile-clicked":
    case "summary-clicked":
    case "capture-clicked":
      root.close()
      break
    }
    root.service.client.call(root.win.winId, name, args || [])
  }
  function switchPanel(direction) {
    return root.bar && typeof root.bar.switchPanelFrom === "function"
      ? root.bar.switchPanelFrom(root.hostWidget || root, direction) : false
  }

  function localizedStatus(status) {
    // mic-active/sys-active describe armed sources, not a running session.
    // refresh_status can send these words even at startup; normalize only
    // those capture claims, leaving diagnostics and lock-transition text intact.
    if (!root.recording && (status === "recording" || status === "mic only")) {
      if (!root.sessionActive) status = "idle"
      else if (root.p("session-paused", false)) status = "paused"
    }
    switch (status) {
    case "idle": return tr("idle")
    case "recording": return tr("recording")
    case "mic only": return tr("mic only")
    case "mic muted": return tr("mic muted")
    case "paused": return tr("paused")
    case "wrapping up": return tr("wrapping up")
    case "start failed": return tr("start failed")
    case "sys ok": return tr("sys ok")
    case "sys quiet": return tr("sys quiet")
    case "sys silent": return tr("sys silent")
    case "sys test failed": return tr("sys test failed")
    case "AI unavailable": return tr("AI unavailable")
    default: return status
    }
  }
  readonly property string presentedStatus: p("stealth-fault", false)
    ? tr("[!] stealth unavailable") : localizedStatus(p("status-text", "idle"))
  // Commons has no separate success/warning role: use accent for success/info,
  // urgent for danger and warning, muted for neutral. Words also convey state.
  readonly property color statusColor: p("stealth-fault", false) ? Commons.Color.urgent
    : !sessionActive && !recording && (p("status-text", "idle") === "recording" || p("status-text", "idle") === "mic only") ? Commons.Color.muted
    : p("status-tone", "neutral") === "danger" || p("status-tone", "neutral") === "warning" ? Commons.Color.urgent
    : p("status-tone", "neutral") === "ok" || p("status-tone", "neutral") === "info" ? Commons.Color.accent : Commons.Color.muted
  readonly property int micPermission: p("mic-permission-state", 0)
  readonly property int micCapture: p("mic-capture-state", 0)
  readonly property string bootstrapMicText: micPermission === 3
    ? (micCapture === 1 ? tr("Mic live") : micCapture === 2 ? tr("Mic failed") : tr("Start mic"))
    : micPermission === 4 ? tr("Requesting...")
    : micPermission === 1 || micPermission === 2 ? tr("Mic denied") : tr("Allow mic")
  readonly property color bootstrapMicColor: micPermission === 1 || micPermission === 2 || micCapture === 2
    ? Commons.Color.urgent : Commons.Color.accent

  function dismissLock() {
    root.localLockOpen = false
    if (root.remoteLockVisible) root.service.client.call(root.lockWin.winId, "dismissed", [])
    if (root.opened) lockButton.forceActiveFocus()
  }
  function selectLock(mode) {
    if (root.remoteLockVisible) root.service.client.call(root.lockWin.winId, "mode-selected", [mode])
    else root.fire("lock-mode-selected", [mode])
    root.localLockOpen = false
    lockButton.forceActiveFocus()
  }
  function stopHolds() {
    if (micRecord) micRecord.finish()
    if (sysRecord) sysRecord.finish()
  }

  onOpenedChanged: {
    if (opened) viewport.contentY = 0
    else { stopHolds(); dismissLock() }
  }
  onReadyChanged: if (!ready) { stopHolds(); localLockOpen = false }
  onRemoteLockVisibleChanged: {
    if (remoteLockVisible) {
      root.open()
      Qt.callLater(function() { unlockedButton.forceActiveFocus() })
    }
  }
  onLockMenuOpenChanged: if (lockMenuOpen) Qt.callLater(function() {
    if (root.opened) unlockedButton.forceActiveFocus()
  })

  Connections {
    target: root.service ? root.service.client : null
    function onInvoked(w, name, args) {
      if (w !== root.lockWin || !w) return
      if (name === "focus") {
        root.localLockOpen = true
        root.open()
        Qt.callLater(function() { unlockedButton.forceActiveFocus() })
      } else if (name === "clear-focus") {
        root.localLockOpen = false
        keyCatcher.forceActiveFocus()
      }
    }
  }

  component Readout: Text {
    textFormat: Text.PlainText
    color: Commons.Color.popups.text
    font.family: Style.font.family
    font.pixelSize: Style.font.body
    wrapMode: Text.Wrap
    Accessible.role: Accessible.StaticText
    Accessible.name: text
  }
  component Action: Bar.ActionButton {
    forceArrow: root.stealth
  }
  component Switch: Bar.ToggleRow {
    forceArrow: root.stealth
  }

  // Conceal immediately; the normal close animation can outlast a capture.
  Binding {
    target: panel
    property: "visible"
    when: !!root.service && (root.service.client.concealed || root.service.client.nativeDialogOpen)
    value: false
  }

  Ui.KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.hostWidget || root
    bar: root.bar
    open: root.opened && !(root.service && (root.service.client.concealed || root.service.client.nativeDialogOpen))
    focusTarget: keyCatcher
    // Match the daemon's suflyor-* stealth layer-rule namespace.
    WlrLayershell.namespace: "suflyor-bar-panel"
    contentWidth: fittedContentWidth(Style.space(440))
    // Fit the actual content, including live status lines and theme spacing.
    contentHeight: fittedContentHeight(Math.ceil(body.implicitHeight))

    FocusScope {
      id: keyCatcher
      objectName: "bar-content"
      anchors.fill: parent
      focus: true
      Keys.onEscapePressed: function(event) {
        if (root.lockMenuOpen) root.dismissLock()
        else root.close()
        event.accepted = true
      }

      Flickable {
        id: viewport
        objectName: "bar-viewport"
        anchors.fill: parent
        anchors.rightMargin: -root.scrollGutter
        contentWidth: width
        contentHeight: body.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
          id: body
          objectName: "bar-body"
          width: Math.max(0, viewport.width - root.scrollGutter)
          spacing: Style.spacing.md

          Row {
            objectName: "bar-heading"
            width: parent.width
            spacing: Style.spacing.controlGap
            Icons.SuflyorMark {
              width: Style.space(28)
              height: width
              ink: root.p("ai-streaming", false) || root.sessionActive || root.recording
                ? Commons.Color.accent : Commons.Color.popups.text
              assistantActive: root.p("ai-streaming", false)
              anchors.verticalCenter: parent.verticalCenter
            }
            Column {
              width: parent.width - x
              anchors.verticalCenter: parent.verticalCenter
              spacing: Style.spacing.xxs
              Readout { text: "Suflyor"; font.pixelSize: Style.font.title; font.bold: true }
              Readout {
                width: parent.width
                text: root.presentedStatus
                color: root.statusColor
                font.pixelSize: Style.font.caption
              }
            }
          }
          // Bootstrap is deliberately exclusive: never expose dead audio/AI controls.
          Column {
            width: parent.width
            visible: root.bootstrap
            enabled: root.ready
            spacing: Style.spacing.rowGap
            Readout { width: parent.width; text: root.tr("Suflyor for macOS"); font.bold: true }
            Readout { width: parent.width; text: root.tr("Native overlay ready; product features are being connected.") }
            Bar.ButtonsRow {
              Action {
                text: root.bootstrapMicText
                selected: root.micCapture === 1
                foreground: root.bootstrapMicColor
                iconText: "\u{f036c}"
                onClicked: root.fire("mic-toggle-clicked")
              }
              Action { text: root.tr("Ask"); iconText: "\u{f03eb}"; onClicked: root.fire("text-ask-clicked") }
            }
            Bar.ButtonsRow {
              Action { text: root.tr("AI setup"); iconText: "\u{f0493}"; onClicked: root.fire("open-settings-clicked") }
              Action { text: root.tr("Quit"); iconText: "\u{f05ad}"; accent: Commons.Color.urgent; onClicked: root.fire("quit-confirm") }
            }
          }

          Column {
            width: parent.width
            visible: !root.bootstrap
            enabled: root.ready
            spacing: Style.spacing.md

            Ui.PanelSectionHeader { text: root.tr("SESSION") }
            Bar.ButtonsRow {
              objectName: "bar-session-row"
              Action {
                objectName: "bar-session-start"
                text: root.p("timer-active", false) ? root.p("timer-label", "00:00") : root.tr("start")
                accessibleLabel: root.p("timer-active", false) ? root.tr("Stop session") + " · " + text : text
                iconText: root.p("timer-active", false) ? "\u{f04db}" : "\u{f040a}"
                selected: root.p("timer-active", false)
                checkable: true
                onClicked: root.fire("timer-toggle-clicked")
              }
              Action {
                visible: root.p("timer-active", false)
                text: root.p("session-paused", false) ? root.tr("Resume the session") : root.tr("Pause the session")
                iconText: root.p("session-paused", false) ? "\u{f040a}" : "\u{f03e4}"
                selected: root.p("session-paused", false)
                checkable: true
                onClicked: root.fire("pause-toggle-clicked")
              }
            }
            Readout {
              width: parent.width
              visible: !root.compact && root.sessionActive && text !== ""
              text: root.p("session-name", "")
              color: Commons.Color.accent
              font.bold: true
            }

            Bar.ButtonsRow {
              visible: !root.compact
              Action {
                text: root.tr("Session archive")
                iconText: "\u{f003c}"
                selected: root.p("archive-open", false)
                checkable: true
                onClicked: root.fire("archive-clicked")
              }
              Action {
                text: root.tr("Meeting summary")
                iconText: root.p("summary-busy", false) ? "\u{f0772}" : "\u{f0219}"
                iconSpinning: root.p("summary-busy", false)
                selected: root.p("summary-busy", false)
                enabled: !root.p("summary-busy", false)
                onClicked: root.fire("summary-clicked")
              }
            }

            Ui.PanelSeparator { visible: !root.compact }
            Ui.PanelSectionHeader { text: root.tr("Audio"); visible: !root.compact }
            Column {
              width: parent.width
              spacing: Style.spacing.sm
              visible: !root.compact
              Switch {
                objectName: "bar-microphone"
                iconName: "mic"
                label: root.tr("Microphone")
                accessibleLabel: root.p("mic-muted", false) ? root.tr("Mic muted (click to unmute)") : root.tr("Mic live (click to mute)")
                checked: !root.p("mic-muted", false)
                onClicked: root.fire("mic-toggle-clicked")
              }
              Switch {
                objectName: "bar-system-audio"
                iconName: "system-audio"
                label: root.tr("System audio")
                accessibleLabel: root.tr("System audio capture")
                checked: root.p("sys-active", false)
                onClicked: root.fire("sys-toggle-clicked")
              }
            }
            // The host's mic-active means unmuted, not that capture is running.
            Readout {
              width: parent.width
              visible: !root.compact
              text: root.tr("Microphone") + " · " + (root.micCapturing ? root.tr("recording")
                : root.sessionActive && root.p("session-paused", false) ? root.tr("paused") : root.tr("idle"))
              color: root.micCapturing ? Commons.Color.accent : Commons.Color.muted
              font.pixelSize: Style.font.caption
            }
            Bar.ButtonsRow {
              Bar.RecordButton {
                id: micRecord
                objectName: "bar-hold-mic"
                text: root.tr("ask")
                iconText: "\u{f036c}"
                recording: root.p("mic-recording", false)
                forceArrow: root.stealth
                onRecordPressed: root.fire("ptt-mic-pressed")
                onRecordReleased: root.fire("ptt-mic-released")
              }
              Bar.RecordButton {
                id: sysRecord
                objectName: "bar-hold-system"
                text: root.tr("grab")
                iconText: "\u{f057e}"
                recording: root.p("sys-recording", false)
                forceArrow: root.stealth
                onRecordPressed: root.fire("ptt-sys-pressed")
                onRecordReleased: root.fire("ptt-sys-released")
              }
            }

            Ui.PanelSeparator { visible: !root.compact }
            Ui.PanelSectionHeader { text: root.tr("AI"); visible: !root.compact }
            Column {
              width: parent.width
              spacing: Style.spacing.sm
              visible: !root.compact
              Switch {
                objectName: "bar-stealth"
                iconName: "stealth"
                label: root.tr("Stealth")
                accessibleLabel: root.tr("Stealth (hide from capture)")
                checked: root.stealth
                foreground: root.p("stealth-fault", false) ? Commons.Color.urgent : Commons.Color.popups.text
                onClicked: root.fire("stealth-toggle-clicked")
              }
              Switch {
                objectName: "bar-auto-tiles"
                iconName: "tiles"
                label: root.tr("Auto-tiles")
                accessibleLabel: root.tr("Aggressive auto-tile")
                checked: root.p("aggressive-active", false)
                onClicked: root.fire("aggressive-toggle-clicked")
              }
            }
            Action {
              id: lockButton
              text: root.deep ? (root.compact || root.p("open-tiles", 0) > 0 ? root.tr("AI off") : root.tr("Local AI unloaded"))
                : root.suppress ? root.tr("Listening") : root.tr("Unlocked")
              accessibleLabel: root.p("lock-a11y", "") || text
              iconText: root.lockIconUnlocked ? "\u{f0340}" : "\u{f033e}"
              accent: root.deep ? Commons.Color.urgent : Commons.Color.accent
              foreground: root.deep ? Commons.Color.urgent : Commons.Color.popups.text
              selected: root.suppress || root.deep
              checkable: true
              // Inline menu: opening a native Slint menu would request an HWND
              // from a headless remote window. Durable transitions stay in Rust.
              onClicked: {
                if (root.lockMenuOpen) root.dismissLock()
                else root.localLockOpen = true
              }
              SequentialAnimation on iconRotation {
                running: root.opened && root.suppress && !root.deep && !Style.reduceMotion
                loops: Animation.Infinite
                NumberAnimation { from: -7; to: 7; duration: Style.duration(85) }
                NumberAnimation { from: 7; to: -7; duration: Style.duration(85) }
                onRunningChanged: if (!running) lockButton.iconRotation = 0
              }
            }
            Column {
              width: parent.width
              visible: root.lockMenuOpen
              spacing: Style.spacing.xxs
              Ui.PanelSectionHeader { text: root.tr("Lock mode") }
              Action {
                id: unlockedButton
                width: parent.width
                leftAlign: true
                text: root.tr("Unlocked")
                iconText: "\u{f0340}"
                checkable: true
                selected: root.lockMode === 0
                foreground: Commons.Color.popups.text
                onClicked: root.selectLock(0)
              }
              Action {
                width: parent.width
                leftAlign: true
                text: root.tr("Listening")
                iconText: "\u{f033e}"
                checkable: true
                selected: root.lockMode === 1
                onClicked: root.selectLock(1)
              }
              Action {
                width: parent.width
                leftAlign: true
                visible: root.lockManaged
                text: root.tr("Unload local AI")
                iconText: "\u{f033e}"
                foreground: Commons.Color.urgent
                accent: Commons.Color.urgent
                checkable: true
                selected: root.lockMode === 2
                onClicked: root.selectLock(2)
              }
            }

            Column {
              width: parent.width
              spacing: Style.spacing.md
              visible: !root.compact && !root.confirming
              Bar.ButtonsRow {
                visible: root.p("open-tiles", 0) > 0 || root.p("can-restore-tile", false)
                Action { text: root.tr("close all"); iconText: "\u{f0156}"; visible: root.p("open-tiles", 0) > 0; onClicked: root.fire("close-all-tiles-clicked") }
                Action { text: root.tr("Restore last tile"); iconText: "\u{f099b}"; visible: root.p("can-restore-tile", false); onClicked: root.fire("restore-tile-clicked") }
              }
              Bar.ButtonsRow {
                Action { text: root.tr("Write a question"); iconText: "\u{f03eb}"; onClicked: root.fire("text-ask-clicked") }
                Action { text: root.tr("+ tile ({})", [root.p("open-tiles", 0)]); onClicked: root.fire("spawn-tile-clicked") }
              }
              Action { text: root.tr("Screenshot to vision"); iconText: "\u{f0100}"; onClicked: root.fire("capture-clicked") }
            }

            Ui.PanelSeparator { visible: !root.compact || root.confirming }
            Ui.PanelSectionHeader { text: root.tr("APP"); visible: !root.compact || root.confirming }
            Bar.ButtonsRow {
              visible: !root.compact && !root.confirming
              Action { objectName: "bar-settings"; text: root.tr("Settings"); iconText: "\u{f0493}"; selected: root.p("settings-open", false); checkable: true; onClicked: root.fire("open-settings-clicked") }
              Action { objectName: "bar-help"; text: root.tr("Help"); iconText: "\u{f02d6}"; selected: root.p("help-open", false); checkable: true; onClicked: root.fire("help-clicked") }
            }
            Bar.ButtonsRow {
              visible: (!root.compact || root.confirming) && (!root.p("restart-armed", false) || !root.p("quit-armed", false))
              Action { text: root.tr("Emergency restart"); iconText: "\u{f0709}"; visible: !root.p("restart-armed", false); onClicked: root.fire("restart-clicked") }
              Action { text: root.tr("Quit"); iconText: "\u{f05ad}"; visible: !root.p("quit-armed", false); accent: Commons.Color.urgent; onClicked: root.fire("quit-clicked") }
            }
            Readout { width: parent.width; text: root.tr("Restart?"); visible: root.p("restart-armed", false); color: Commons.Color.urgent }
            Bar.ButtonsRow {
              visible: root.p("restart-armed", false)
              Action { text: root.tr("Yes"); accent: Commons.Color.urgent; selected: true; onClicked: root.fire("restart-confirm") }
              Action { text: root.tr("No"); onClicked: root.fire("restart-cancel") }
            }
            Readout { width: parent.width; text: root.tr("Quit?"); visible: root.p("quit-armed", false); color: Commons.Color.urgent }
            Bar.ButtonsRow {
              visible: root.p("quit-armed", false)
              Action { text: root.tr("Yes"); accent: Commons.Color.urgent; selected: true; onClicked: root.fire("quit-confirm") }
              Action { text: root.tr("No"); onClicked: root.fire("quit-cancel") }
            }
            Action {
              visible: !root.confirming
              text: root.compact ? root.tr("Expand the bar") : root.tr("Compact reader mode")
              iconText: root.compact ? "\u{f0293}" : "\u{f0294}"
              checkable: true
              selected: root.compact
              // Compact still has meaning: collapse secondary panel groups,
              // retaining timer/pause, holds, lock, telemetry and Expand.
              onClicked: root.fire("compact-toggle-clicked")
            }
            // No Hide to tray here: Omarchy's bar widget is the restore surface.
            // tray-available and hide-to-tray-clicked describe a native Slint bar.
          }

          Ui.PanelSeparator {}
          Readout {
            width: parent.width
            visible: !root.bootstrap && text !== ""
            text: root.p("active-stack", "") + (root.p("tok-per-sec", "") !== "" ? " · " + root.p("tok-per-sec", "") : "")
            color: Commons.Color.accent
            font.pixelSize: Style.font.caption
            font.bold: true
          }
          Flow {
            width: parent.width
            spacing: Style.spacing.controlGap
            visible: !root.bootstrap && !root.compact
            Readout { visible: root.p("app-memory", "") !== ""; text: root.tr("App RAM") + " " + root.p("app-memory", ""); color: Commons.Color.muted; font.pixelSize: Style.font.caption }
            Readout { visible: root.p("mlx-memory", "") !== ""; text: root.tr("MLX") + " " + root.p("mlx-memory", ""); color: Commons.Color.muted; font.pixelSize: Style.font.caption }
          }
          Ui.PanelSectionHeader { text: root.tr("Transcript") }
          Row {
            objectName: "bar-transcript"
            width: parent.width
            spacing: Style.spacing.controlGap
            Rectangle {
              width: Style.space(7)
              height: width
              radius: width / 2
              y: Style.space(5)
              color: root.p("last-transcript-source", "") === "mic" ? Commons.Color.accent
                : root.p("last-transcript-source", "") === "sys" ? Commons.Color.foreground : Commons.Color.muted
              Accessible.role: Accessible.StaticText
              Accessible.name: root.p("last-transcript-source", "") === "mic" ? root.tr("Microphone")
                : root.p("last-transcript-source", "") === "sys" ? root.tr("System audio") : ""
            }
            Readout {
              width: parent.width - x
              text: root.p("last-transcript-line", "") || root.tr("Waiting for speech…")
              color: root.p("last-transcript-line", "") === "" ? Commons.Color.muted : Commons.Color.popups.text
              maximumLineCount: 3
              elide: Text.ElideRight
            }
          }
          Readout {
            visible: !root.bootstrap && !root.compact && root.p("ai-streaming", false)
            text: root.tr("AI streaming")
            color: Commons.Color.accent
            font.pixelSize: Style.font.caption
          }
        }
      }
    }
  }
}
