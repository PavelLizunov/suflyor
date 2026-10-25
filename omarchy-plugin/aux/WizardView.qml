pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "../tile" as Tile
import "small" as Small

Item {
  id: root
  property var win
  property var service
  function p(name, fallback) { return win ? win.prop(name, fallback) : fallback }
  function set(name, value) { if (win && service) service.client.set(win.winId, name, value) }
  function call(name, args) { if (win && service) service.client.call(win.winId, name, args || []) }
  function tr(text, args) { return service ? service.tr(text, args || []) : "" }
  Accessible.name: root.tr("Setup wizard")
  readonly property int step: p("step", 0)
  onStepChanged: if (cardViewport && cardViewport.contentItem) cardViewport.contentItem.contentY = 0
  readonly property var checks: [
    { step: 1, title: tr("AI"), level: p("ai-level", 3), detail: p("ai-detail", ""),
      summary: p("summary-ai", ""), button: tr("Test AI"), callback: "ai-test-clicked",
      description: tr("The AI is chosen in Settings → AI providers. Here we only check that it answers.") },
    { step: 2, title: tr("Speech recognition (STT)"), level: p("stt-level", 3), detail: p("stt-detail", ""),
      summary: p("summary-stt", ""), button: tr("Test STT"), callback: "stt-test-clicked",
      description: tr("Speech recognition is set up in Settings → STT. Here we only check the connection.") },
    { step: 3, title: tr("Microphone"), level: p("mic-level", 3), detail: p("mic-detail", ""),
      summary: p("summary-mic", ""), button: tr("Record test"), callback: "mic-test-clicked",
      description: tr("Records 3 seconds. Speak during the test to see your level.") },
    { step: 4, title: tr("System audio"), level: p("sys-level", 3), detail: p("sys-detail", ""),
      summary: p("summary-sys", ""), button: tr("Check system audio"), callback: "sys-test-clicked",
      description: tr("Needed to hear the other side. Plays a short sound and checks that it is recorded.") }
  ]
  readonly property bool needsAttention: checks.some(function(check) { return check.level === 4 })
  function levelText(level) {
    if (level === -1) return tr("checking…")
    if (level === 0) return tr("OK")
    if (level === 2) return tr("Not configured")
    if (level === 4) return tr("Needs attention")
    return tr("—")
  }
  function levelColor(level) {
    return level === 0 ? Commons.Color.accent : level === 4 ? Commons.Color.urgent : Commons.Color.muted
  }
  readonly property var rail: [
    { label: tr("Mode"), level: 0 },
    { label: tr("AI"), level: p("ai-level", 3) },
    { label: tr("STT"), level: p("stt-level", 3) },
    { label: tr("Microphone"), level: p("mic-level", 3) },
    { label: tr("System audio"), level: p("sys-level", 3) },
    { label: tr("Stealth"), level: 0 },
    { label: tr("Done"), level: 3 }
  ]
  // drag-start-requested / drag-moved are native-window chrome, owned by AuxLayer.
  Small.Surface {
    anchors.fill: parent
    view: root
    closeCallback: "cancelled"
    title: root.tr("Setup wizard")
    note: root.tr("Step {} of 7", [root.step + 1])
    RowLayout {
      anchors.fill: parent
      spacing: Commons.Style.spacing.lg
      Controls.ScrollView {
        id: railViewport
        Layout.preferredWidth: Commons.Style.space(164)
        Layout.fillHeight: true
        contentWidth: availableWidth
        Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
        ColumnLayout {
          width: Math.max(0, railViewport.availableWidth - Commons.Style.spacing.sm)
          spacing: Commons.Style.spacing.xs
          Tile.Label {
            text: root.tr("Setup")
            color: Commons.Color.muted
            font.pixelSize: Commons.Style.font.caption
            font.bold: true
          }
          Repeater {
            model: root.rail
            delegate: Rectangle {
              id: railStep
              required property var modelData
              required property int index
              readonly property bool current: root.step === index
              readonly property bool done: index < 6 && root.step > index
              readonly property bool passed: done && modelData.level === 0
              readonly property bool attention: done && modelData.level === 4
              Layout.fillWidth: true
              implicitHeight: Math.max(Commons.Style.space(32), label.implicitHeight + Commons.Style.spacing.sm * 2)
              radius: Commons.Style.cornerRadius
              color: current ? Commons.Style.selectedAccentFill : "transparent"
              Behavior on color { ColorAnimation { duration: Commons.Style.duration(120) } }
              Rectangle {
                visible: railStep.current
                width: Commons.Style.space(3)
                height: Math.max(0, parent.height - Commons.Style.spacing.sm * 2)
                y: Commons.Style.spacing.sm
                color: Commons.Color.accent
              }
              RowLayout {
                anchors.fill: parent
                anchors.margins: Commons.Style.spacing.sm
                spacing: Commons.Style.spacing.sm
                Tile.Label {
                  Layout.preferredWidth: Commons.Style.space(22)
                  horizontalAlignment: Text.AlignHCenter
                  text: railStep.passed ? "\uf00c" : railStep.attention ? "\uf12a" : root.tr("{}", [railStep.index + 1])
                  color: railStep.attention ? Commons.Color.urgent : railStep.passed || railStep.current ? Commons.Color.accent : Commons.Color.muted
                  font.bold: true
                }
                Tile.Label {
                  id: label
                  Layout.fillWidth: true
                  Layout.minimumWidth: 0
                  text: railStep.modelData.label
                  color: railStep.current ? Commons.Color.accent : railStep.done ? Commons.Color.foreground : Commons.Color.muted
                }
              }
            }
          }
        }
      }
      Rectangle { Layout.fillHeight: true; Layout.preferredWidth: Commons.Style.space(1); color: Commons.Style.normalBorderColor }
      ColumnLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumWidth: 0
        spacing: Commons.Style.spacing.lg
        Rectangle {
          Layout.fillWidth: true
          Layout.fillHeight: true
          Layout.minimumHeight: 0
          color: Commons.Style.normalFill
          border.color: Commons.Style.normalBorderColor
          radius: Commons.Style.cornerRadius
          Controls.ScrollView {
            id: cardViewport
            anchors.fill: parent
            anchors.margins: Commons.Style.spacing.lg
            contentWidth: availableWidth
            Controls.ScrollBar.horizontal.policy: Controls.ScrollBar.AlwaysOff
            ColumnLayout {
              width: Math.max(0, cardViewport.availableWidth - Commons.Style.spacing.lg)
              spacing: Commons.Style.spacing.lg
              ColumnLayout {
                visible: root.step === 0
                Layout.fillWidth: true
                spacing: Commons.Style.spacing.sm
                Tile.Label {
                  Layout.fillWidth: true
                  text: root.tr("Choose how Suflyor works")
                  font.pixelSize: Commons.Style.font.heading
                  font.bold: true
                }
                Repeater {
                  model: [
                    { title: root.tr("Cloud"), description: root.tr("Speech recognition and AI in the cloud: a quick start, needs keys and internet.") },
                    { title: root.tr("Fully local"), description: root.tr("AI and speech on this PC, offline; download models first.") },
                    { title: root.tr("Mixed"), description: root.tr("For example, speech on this PC and AI in the cloud; set up in Settings.") }
                  ]
                  delegate: Small.ModeOption {
                    id: option
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    title: modelData.title
                    description: modelData.description
                    selected: root.p("mode", 0) === index
                    onChosen: {
                      root.set("mode", option.index)
                      root.call("mode-selected", [option.index])
                    }
                  }
                }
              }
              Repeater {
                model: root.checks
                delegate: ColumnLayout {
                  id: checkPanel
                  required property var modelData
                  visible: root.step === modelData.step
                  Layout.fillWidth: true
                  spacing: Commons.Style.spacing.lg
                  Tile.Label {
                    Layout.fillWidth: true
                    text: checkPanel.modelData.title
                    font.pixelSize: Commons.Style.font.heading
                    font.bold: true
                  }
                  Tile.Label { Layout.fillWidth: true; text: checkPanel.modelData.description }
                  RowLayout {
                    Layout.fillWidth: true
                    spacing: Commons.Style.spacing.sm
                    Small.Action {
                      text: checkPanel.modelData.button
                      iconText: "\uf021"
                      onClicked: root.call(checkPanel.modelData.callback)
                    }
                    Small.Action {
                      visible: checkPanel.modelData.step === 1
                      text: root.tr("Install the built-in AI")
                      iconText: "\uf019"
                      onClicked: root.call("install-local-clicked")
                    }
                  }
                  RowLayout {
                    Layout.fillWidth: true
                    spacing: Commons.Style.spacing.sm
                    Tile.Label {
                      Layout.alignment: Qt.AlignTop
                      Layout.maximumWidth: checkPanel.width * 0.38
                      text: root.levelText(checkPanel.modelData.level)
                      color: root.levelColor(checkPanel.modelData.level)
                      font.bold: true
                    }
                    Tile.Label {
                      Layout.fillWidth: true
                      text: root.tr("{}", [checkPanel.modelData.detail])
                    }
                  }
                  Tile.Label {
                    // suflyord's failure text already names the cause and the button; this
                    // covers only a failure that came without one.
                    visible: checkPanel.modelData.level === 4 && checkPanel.modelData.detail === ""
                    Layout.fillWidth: true
                    color: Commons.Color.urgent
                    text: root.tr("The check did not provide a failure reason. Press \"{}\" to try again.", [checkPanel.modelData.button])
                  }
                }
              }
              ColumnLayout {
                visible: root.step === 5
                Layout.fillWidth: true
                spacing: Commons.Style.spacing.lg
                Tile.Label {
                  Layout.fillWidth: true
                  text: root.tr("Stealth")
                  font.pixelSize: Commons.Style.font.heading
                  font.bold: true
                }
                RowLayout {
                  Layout.fillWidth: true
                  spacing: Commons.Style.spacing.sm
                  Tile.Label {
                    id: stealthLabel
                    Layout.fillWidth: true
                    text: root.tr("Hide Suflyor windows from screen capture")
                  }
                  Ui.ToggleSwitch {
                    id: stealth
                    checked: root.p("stealth-on", false)
                    activeFocusOnTab: true
                    Accessible.role: Accessible.CheckBox
                    Accessible.name: stealthLabel.text
                    Accessible.checked: checked
                    function toggle() {
                      var next = !checked
                      root.set("stealth-on", next)
                      root.call("stealth-toggled", [next])
                    }
                    onToggled: toggle()
                    Keys.onSpacePressed: toggle()
                    Keys.onReturnPressed: toggle()
                    Keys.onEnterPressed: toggle()
                    hasCursor: activeFocus
                  }
                }
                Tile.Label {
                  Layout.fillWidth: true
                  text: root.tr("When it is on: press Print Screen or open a screen-share preview. Suflyor windows should disappear from the capture and stay on your screen. This cannot be checked automatically.")
                }
              }
              ColumnLayout {
                visible: root.step === 6
                Layout.fillWidth: true
                spacing: Commons.Style.spacing.lg
                Tile.Label {
                  Layout.fillWidth: true
                  text: root.tr(root.needsAttention ? "Almost done: check the marked items" : "Done")
                  font.pixelSize: Commons.Style.font.heading
                  font.bold: true
                }
                Repeater {
                  model: root.checks
                  delegate: RowLayout {
                    id: summaryRow
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Commons.Style.spacing.sm
                    Tile.Label {
                      Layout.preferredWidth: Commons.Style.space(100)
                      Layout.alignment: Qt.AlignTop
                      text: summaryRow.modelData.step === 2 ? root.tr("STT") : summaryRow.modelData.title
                      color: Commons.Color.muted
                    }
                    Tile.Label {
                      Layout.preferredWidth: Commons.Style.space(90)
                      Layout.alignment: Qt.AlignTop
                      text: root.levelText(summaryRow.modelData.level)
                      color: root.levelColor(summaryRow.modelData.level)
                      font.bold: true
                    }
                    Tile.Label {
                      Layout.fillWidth: true
                      text: root.tr("{}", [summaryRow.modelData.summary])
                    }
                  }
                }
                Small.Action {
                  text: root.tr("Open diagnostics")
                  iconText: "\uf0f1"
                  onClicked: root.call("open-diagnostics")
                }
              }
            }
          }
        }
        RowLayout {
          Layout.fillWidth: true
          spacing: Commons.Style.spacing.sm
          Small.Action {
            text: root.tr("Back")
            iconText: "\uf060"
            enabled: root.step > 0
            onClicked: root.call("nav-back")
          }
          Small.Action {
            visible: root.step < 6
            text: root.tr("Skip")
            iconText: "\uf051"
            onClicked: root.call("nav-skip")
          }
          Small.Action {
            visible: root.step < 6
            text: root.tr("Next")
            iconText: "\uf061"
            selected: true
            onClicked: root.call("nav-next")
          }
          Small.Action {
            visible: root.step === 6
            text: root.tr("Start working")
            iconText: "\uf04b"
            selected: true
            onClicked: root.call("finished")
          }
        }
      }
    }
  }
}
