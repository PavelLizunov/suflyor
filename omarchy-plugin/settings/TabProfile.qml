pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import "controls" as C

// SettingsWindow tab 10. Remote state is authoritative, including async dictation/AI results.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property var profileNames: win ? win.prop("profile-names", []) : []
  readonly property int profileIndex: win ? win.prop("active-profile-index", -1) : -1
  readonly property string profileName: win ? win.prop("profile-name-input", "") : ""
  readonly property string context: win ? win.prop("meeting-context-input", "") : ""
  readonly property bool dictating: win ? win.prop("context-dictating", false) : false
  readonly property bool processing: win ? win.prop("context-processing", false) : false
  readonly property string result: win ? win.prop("meeting-context-result", "") : ""
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    objectName: "profile-card"
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Persistent context sent with every AI request — your role, focus areas, vocabulary. Saved to config; applied to new AI calls immediately.", []) : ""
      font.pixelSize: Style.font.body
    }
    // The empty-state hint keeps a full row instead of squeezing the name field.
    RowLayout {
      objectName: "profile-picker-row"
      width: parent.width
      spacing: Style.space(6)
      C.SettingsCaption {
        Layout.preferredWidth: implicitWidth
        Layout.fillWidth: false
        text: root.service ? root.service.tr("Profile:", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsComboBox {
        Layout.fillWidth: true
        Layout.preferredWidth: Style.space(170)
        Layout.minimumWidth: 0
        visible: root.profileNames.length > 0
        model: root.profileNames
        currentIndex: root.profileIndex
        onSelected: function(index) {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "active-profile-index", index)
          root.service.client.call(root.win.winId, "profile-selected", [index])
        }
      }
      C.SettingsLineEdit {
        objectName: "profile-name-field"
        Layout.fillWidth: true
        Layout.preferredWidth: Style.space(150)
        Layout.minimumWidth: 0
        placeholderText: root.service ? root.service.tr("profile name", []) : ""
        text: root.profileName
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "profile-name-input", text)
      }
    }
    C.SettingsCaption {
      visible: root.profileNames.length === 0
      text: root.service ? root.service.tr("None yet — enter a name and click 'Add profile'.", []) : ""
      font.pixelSize: Style.font.bodySmall
    }
    C.SettingsButtonsRow {
      C.SettingsButton {
        objectName: "profile-add"
        iconName: "plus"
        text: root.service ? root.service.tr("Add profile", []) : ""
        onClicked: {
          if (!root.win || !root.service) return
          root.service.client.call(root.win.winId, "profile-add", [root.profileName])
          root.service.client.set(root.win.winId, "profile-name-input", "")
        }
      }
      C.SettingsButton {
        objectName: "profile-rename"
        iconName: "pencil"
        text: root.service ? root.service.tr("Rename", []) : ""
        enabled: root.profileIndex >= 0 && root.profileName !== ""
        onClicked: {
          if (!root.win || !root.service) return
          root.service.client.call(root.win.winId, "profile-rename", [root.profileName])
          root.service.client.set(root.win.winId, "profile-name-input", "")
        }
      }
      C.SettingsButton {
        objectName: "profile-delete"
        iconName: "trash"
        text: root.service ? root.service.tr("Delete", []) : ""
        enabled: root.profileIndex >= 0
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "profile-delete", [])
      }
    }
    C.SettingsTextEdit {
      text: root.context
      placeholderText: root.service ? root.service.tr("e.g. SRE interview; brief answers, AWS examples.", []) : ""
      onEdited: function(text) {
        if (root.win && root.service && text !== root.context)
          root.service.client.set(root.win.winId, "meeting-context-input", text)
      }
    }
    C.SettingsButtonsRow {
      C.SettingsButton {
        iconName: root.dictating ? "stop" : "mic"
        text: root.service ? root.service.tr(root.dictating ? "Stop recording" : "Dictate", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "context-dictate-clicked", [])
      }
      C.SettingsButton {
        iconName: "check"
        text: root.service ? root.service.tr("Save context", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "meeting-context-save", [root.context])
      }
      C.SettingsButton {
        iconName: "ai"
        text: root.service ? root.service.tr(root.processing ? "Structuring…" : "Structure via AI", []) : ""
        enabled: !root.processing
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "context-process-clicked", [])
      }
    }
    C.StatusLine {
      text: root.result
      tone: "accent"
      font.pixelSize: Style.font.body
    }
  }
}
