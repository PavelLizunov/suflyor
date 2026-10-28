pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "../../tile" as Tile

ColumnLayout {
  id: root
  required property var view
  readonly property bool hasResult: view.p("has-diarization", false)
  readonly property bool busy: view.p("diarizing", false)
  // Renaming rebuilds the wire array on each keystroke. Keep editors alive
  // while speaker identities are unchanged so typing does not lose focus.
  readonly property string speakerKeys: JSON.stringify(view.p("speakers", []).map(function(speaker) { return speaker.id }))
  spacing: Commons.Style.spacing.sm
  RowLayout {
    visible: root.view.p("nemotron-available", false)
    Layout.fillWidth: true
    spacing: Commons.Style.spacing.sm
    Ui.Dropdown {
      Layout.fillWidth: true
      options: [root.view.service.tr("NVIDIA Nemotron 3", []), root.view.service.tr("Pyannote + WeSpeaker", [])]
      value: options[root.view.p("use-nemotron", false) ? 0 : 1]
      enabled: !root.busy && !root.view.p("installing-diar-models", false)
      onChanged: function(next) {
        root.view.call("select-diar-engine", [options.indexOf(next) === 0])
        value = Qt.binding(function() { return options[root.view.p("use-nemotron", false) ? 0 : 1] })
      }
    }
    Action {
      visible: root.busy && root.view.p("use-nemotron", false)
      text: root.view.service.tr("Cancel", [])
      onClicked: root.view.call("cancel-diarization")
    }
  }
  Tile.Label {
    visible: root.view.p("use-nemotron", false)
    Layout.fillWidth: true
    text: root.view.service ? root.view.service.tr("Up to 8 speakers automatically. GPU preferred, CPU fallback.", []) : ""
    color: Commons.Color.muted
  }
  RowLayout {
    visible: root.hasResult
    Layout.fillWidth: true
    spacing: Commons.Style.spacing.sm
    Action {
      text: root.view.service ? root.view.service.tr("By role", []) : ""
      iconText: "\uf007"
      selected: !root.view.p("by-voice", false)
      onClicked: root.view.call("toggle-by-voice", [false])
    }
    Action {
      text: root.view.service ? root.view.service.tr("By voice", []) : ""
      iconText: "\uf0c0"
      selected: root.view.p("by-voice", false)
      onClicked: root.view.call("toggle-by-voice", [true])
    }
  }
  Tile.Label {
    visible: root.view.p("timeline-unreliable", false) && root.hasResult
    Layout.fillWidth: true
    text: root.view.service ? root.view.service.tr("Speaker timings may be approximate for this recording.", []) : ""
    color: Commons.Color.urgent
  }
  ColumnLayout {
    visible: root.view.p("needs-diar-models", false) && !root.view.p("can-diarize", false)
    Layout.fillWidth: true
    spacing: Commons.Style.spacing.sm
    Tile.Label {
      Layout.fillWidth: true
      text: root.view.service ? root.view.service.tr(root.view.p("use-nemotron", false) ? "Nemotron requires a one-time model download (~102 MB)." : "Speaker detection needs a one-time model download (~30 MB).", []) : ""
      color: Commons.Color.muted
    }
    Action {
      enabled: !root.view.p("installing-diar-models", false)
      text: root.view.service ? root.view.service.tr(root.view.p("installing-diar-models", false)
        ? "Downloading models…" : "Install speaker models", []) : ""
      iconText: "\uf019"
      onClicked: root.view.call("install-diar-models")
    }
  }
  RowLayout {
    visible: root.view.p("can-diarize", false)
    Layout.fillWidth: true
    spacing: Commons.Style.spacing.sm
    Tile.Label {
      visible: !root.view.p("use-nemotron", false)
      text: root.view.service ? root.view.service.tr("Speakers (besides you):", []) : ""
      color: Commons.Color.muted
    }
    // Count is an editable field: its stepper buttons intentionally stay compact.
    Tile.ActionButton {
      visible: !root.view.p("use-nemotron", false)
      iconText: "\uf068"
      label: root.view.service ? root.view.service.tr("Speakers (besides you):", []) : ""
      onClicked: if (root.view.p("speaker-count", 2) > 0) root.view.set("speaker-count", root.view.p("speaker-count", 2) - 1)
    }
    Tile.Label {
      visible: !root.view.p("use-nemotron", false)
      Layout.preferredWidth: Commons.Style.space(48)
      text: root.view.p("speaker-count", 2) === 0
        ? (root.view.service ? root.view.service.tr("Auto", []) : "") : String(root.view.p("speaker-count", 2))
      font.bold: true
      horizontalAlignment: Text.AlignHCenter
    }
    Tile.ActionButton {
      visible: !root.view.p("use-nemotron", false)
      iconText: "\uf067"
      label: root.view.service ? root.view.service.tr("Speakers (besides you):", []) : ""
      onClicked: if (root.view.p("speaker-count", 2) < 8) root.view.set("speaker-count", root.view.p("speaker-count", 2) + 1)
    }
    Action {
      enabled: !root.busy
      iconText: "\uf0c0"
      text: root.view.service ? root.view.service.tr(root.hasResult ? "Re-detect speakers" : "Detect speakers", []) : ""
      onClicked: {
        if (root.hasResult && root.view.p("has-speaker-names", false)) root.view.set("confirm-rediar", true)
        else root.view.call("run-diarization")
      }
    }
  }
  Tile.Label {
    visible: root.view.p("diar-status", "") !== ""
    Layout.fillWidth: true
    text: root.view.p("diar-status", "")
    color: Commons.Color.muted
  }
  Controls.ScrollView {
    id: speakers
    visible: root.view.p("by-voice", false) && root.view.p("speakers", []).length > 0
    Layout.fillWidth: true
    Layout.preferredHeight: names.implicitHeight + Commons.Style.space(14)
    contentHeight: availableHeight
    clip: true
    Controls.ScrollBar.vertical.policy: Controls.ScrollBar.AlwaysOff
    RowLayout {
      id: names
      spacing: Commons.Style.spacing.lg
      Repeater {
        model: JSON.parse(root.speakerKeys)
        RowLayout {
          id: speaker
          required property int index
          readonly property var speakerData: root.view.p("speakers", [])[index] || ({})
          spacing: Commons.Style.spacing.sm
          Rectangle {
            Layout.preferredWidth: Commons.Style.space(11)
            Layout.preferredHeight: Commons.Style.space(11)
            radius: width / 2
            color: root.view.wireColor(speaker.speakerData.color)
          }
          Ui.TextField {
            Layout.preferredWidth: Commons.Style.space(130)
            text: speaker.speakerData.label || ""
            enabled: !root.busy
            onTextEdited: root.view.call("rename-speaker", [speaker.speakerData.id, text])
          }
        }
      }
    }
  }
}
