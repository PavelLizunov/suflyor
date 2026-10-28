pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import "controls" as C

// SettingsWindow tab 15. ComponentRow keeps its wire-format kebab-case keys.
Column {
  id: root
  property var win: null
  property var service: null
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  function p(name, fallback) { return root.win ? root.win.prop(name, fallback) : fallback }
  function tr(msgid, args) { return root.service ? root.service.tr(msgid, args || []) : "" }
  function call(name, args) {
    if (root.win && root.service) root.service.client.call(root.win.winId, name, args || [])
  }
  function openTab(tab) {
    if (!root.win || !root.service) return
    root.service.client.set(root.win.winId, "active-tab", tab)
    root.call("tab-selected", [tab])
  }
  function installStatus() {
    var phase = root.p("component-busy-phase", 0)
    var label = root.p("component-busy-label", "")
    return phase === 1 ? root.tr("Preparing…")
      : phase === 2 ? root.tr("Downloading: {}…", [label])
      : phase === 3 ? root.tr("Verifying: {}…", [label])
      : phase === 4 ? root.tr("Unpacking: {}…", [label])
      : phase === 5 ? root.tr("{} — already installed", [label])
      : phase === 6 ? root.tr("Voices installed")
      : phase === 7 ? root.tr("Installed partially; failed: {}", [label])
      : phase === 9 ? root.tr("{} — download failed (retry later)", [label])
      : phase === 10 ? root.tr("Downloading the engine (~53 MB)…")
      : phase === 11 ? root.tr("Verifying…")
      : phase === 12 ? root.tr("Unpacking…")
      : phase === 13 ? root.tr("Text recognition already installed")
      : phase === 14 ? root.tr("Text recognition installed")
      : root.tr("Couldn't install — check the connection and retry.")
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.tr("In use now")
    Repeater {
      model: root.p("active-rows", [])
      delegate: C.WrapBlock {
        id: activeRow
        required property var modelData
        RowLayout {
          width: parent.width
          spacing: Style.space(10)
          Rectangle {
            Layout.preferredWidth: Style.space(10)
            Layout.preferredHeight: Style.space(10)
            radius: width / 2
            color: activeRow.modelData.installed ? Commons.Color.accent : Commons.Color.urgent
          }
          Column {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            spacing: Style.space(4)
            C.SettingsCaption {
              text: activeRow.modelData.name
              color: Commons.Color.foreground
              font.pixelSize: Style.font.body
              font.bold: true
            }
            C.SettingsCaption { text: activeRow.modelData.installed ? activeRow.modelData.detail : activeRow.modelData.hint }
          }
        }
        C.SettingsButton {
          visible: !activeRow.modelData.installed && activeRow.modelData["jump-tab"] >= 0
          text: root.tr("Set up")
          onClicked: root.openTab(activeRow.modelData["jump-tab"])
        }
        C.StatusLine {
          visible: activeRow.modelData.installed
          text: root.tr("set up")
          tone: "good"
          font.bold: true
        }
      }
    }
  }
  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.tr("Parts for working without the internet. Each is needed only when its tab selects it, and downloads on demand: here or in the tab named under it.")
      font.pixelSize: Style.font.bodySmall
    }
    Repeater {
      model: root.p("component-rows", [])
      delegate: C.SettingsCard {
        id: componentRow
        required property var modelData
        required property int index
        readonly property bool busy: index === root.p("component-busy-index", -1)
        readonly property bool canInstall: modelData.installable && !modelData.installed
        readonly property bool canOpen: !modelData.installed && modelData.needed && modelData["jump-tab"] >= 0
        RowLayout {
          width: parent.width
          spacing: Style.space(10)
          Rectangle {
            Layout.preferredWidth: Style.space(10)
            Layout.preferredHeight: Style.space(10)
            radius: width / 2
            color: componentRow.modelData.installed ? Commons.Color.accent : Commons.Color.muted
          }
          Column {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            spacing: Style.space(4)
            C.SettingsCaption {
              text: componentRow.modelData.name
              color: Commons.Color.foreground
              font.pixelSize: Style.font.body
              font.bold: true
            }
            C.SettingsCaption {
              text: componentRow.modelData.installed
                ? (componentRow.modelData.detail !== "" ? componentRow.modelData.detail : root.tr("installed"))
                : componentRow.modelData.hint
            }
          }
        }
        C.StatusLine {
          visible: componentRow.busy
          text: root.installStatus()
          tone: root.p("component-busy-phase", 0) === 7 || root.p("component-busy-phase", 0) === 9 || root.p("component-busy-phase", 0) === 8 || root.p("component-busy-phase", 0) === 0 ? "warning" : "accent"
        }
        C.SettingsButtonsRow {
          visible: !componentRow.busy && (componentRow.canInstall || componentRow.canOpen)
          C.SettingsButton {
            visible: componentRow.canInstall
            text: root.tr("Install")
            onClicked: root.call("component-install", [componentRow.index])
          }
          C.SettingsButton {
            visible: componentRow.canOpen
            text: root.tr("Open")
            onClicked: root.openTab(componentRow.modelData["jump-tab"])
          }
        }
        C.StatusLine {
          visible: !componentRow.busy && !componentRow.canInstall && !componentRow.canOpen
          text: root.tr(componentRow.modelData.installed ? "installed" : componentRow.modelData.needed ? "not installed" : "not needed")
          tone: componentRow.modelData.installed ? "good" : "muted"
          font.bold: true
        }
      }
    }
  }
  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.tr("Speaker detection")
    C.SettingsComboBox {
      visible: root.p("diar-nemotron-supported", false)
      model: [root.tr("NVIDIA Nemotron 3"), root.tr("Pyannote + WeSpeaker")]
      currentIndex: root.p("diar-use-nemotron", false) ? 0 : 1
      enabled: !root.p("diar-installing", false)
      onSelected: function(index) { root.call("diar-engine-selected", [index === 0]) }
    }
    C.SettingsCaption {
      text: root.tr("Choose the speaker detection model. NVIDIA Nemotron 3 detects up to 8 voices automatically; Pyannote + WeSpeaker also accepts a speaker count.")
      font.pixelSize: Style.font.bodySmall
    }
    C.SettingsButton {
      visible: !root.p("diar-models-installed", false)
      text: root.tr(root.p("diar-installing", false) ? "Installing…" : "Install speaker models")
      enabled: !root.p("diar-installing", false)
      onClicked: root.call("diar-install-clicked")
    }
    C.StatusLine {
      visible: root.p("diar-models-installed", false) && root.p("diar-install-status", "") === ""
      text: root.tr("Speaker models are installed.")
    }
    C.StatusLine { text: root.p("diar-install-status", "") }
  }
}
