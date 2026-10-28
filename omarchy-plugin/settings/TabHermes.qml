pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import "controls" as C

// SettingsWindow tab 16. Fine tuning stays in the remote mirror across tab changes.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property bool moreOpen: win ? win.prop("hermes-more-open", false) : false
  readonly property string apiResult: win ? win.prop("hermes-api-test-result", "") : ""
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(12)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsCaption {
      text: root.service ? root.service.tr("Bridge: {}. API: {}", [root.win ? root.win.prop("hermes-bridge-status", "") : "", root.apiResult === "" ? root.service.tr("not checked", []) : root.apiResult]) : ""
      font.pixelSize: Style.font.body
    }
    C.DarkCheck {
      text: root.service ? root.service.tr("Bridge for Hermes", []) : ""
      checked: root.win ? root.win.prop("hermes-bridge-enabled", false) : false
      onToggled: function(checked) {
        if (!root.win || !root.service) return
        root.service.client.set(root.win.winId, "hermes-bridge-enabled", checked)
        root.service.client.call(root.win.winId, "hermes-bridge-toggled", [checked])
      }
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Install plugin in Hermes", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "hermes-plugin-install", [])
    }
    C.StatusLine {
      text: root.win ? root.win.prop("hermes-plugin-install-status", "") : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsButtonsRow {
      C.SettingsButton {
        text: root.service ? root.service.tr("Get key from local Hermes", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "hermes-api-setup", [])
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Check", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "hermes-api-test", [])
      }
    }
    C.StatusLine {
      text: root.win ? root.win.prop("hermes-api-setup-status", "") : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("After installing the plugin or getting a key, restart Hermes.", []) : ""
    }
    C.SettingsButton {
      text: root.service ? root.service.tr(root.moreOpen ? "Hide fine tuning" : "Show fine tuning", []) : ""
      onClicked: if (root.win && root.service) root.service.client.set(root.win.winId, "hermes-more-open", !root.moreOpen)
    }
  }

  C.WrapBlock {
    width: Math.max(0, root.width - root.padding * 2)
    visible: root.moreOpen
    C.SettingsCard {
      title: root.service ? root.service.tr("Hermes — bridge (agent reads suflyor)", []) : ""
      C.SettingsCaption {
        text: root.service ? root.service.tr("Bind host — leave 127.0.0.1 for a Hermes on THIS machine; for a REMOTE Hermes over Tailscale set this machine's Tailscale IP (100.x) or 0.0.0.0:", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsLineEdit {
        placeholderText: root.service ? root.service.tr("127.0.0.1", []) : ""
        text: root.win ? root.win.prop("hermes-bridge-host", "127.0.0.1") : "127.0.0.1"
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "hermes-bridge-host", text)
      }
      C.StatusLine {
        visible: root.win ? root.win.prop("hermes-bridge-remote", false) : false
        text: root.service ? root.service.tr("Bridge is reachable off this machine — use a trusted network (Tailscale) only; a token is required.", []) : ""
        tone: "warning"
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Port:", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsLineEdit {
        placeholderText: root.service ? root.service.tr("8654", []) : ""
        text: root.win ? root.win.prop("hermes-bridge-port", "8654") : "8654"
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "hermes-bridge-port", text)
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Token (plugin installation writes it into Hermes):", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsFieldRow {
        C.SettingsLineEdit {
          Layout.fillWidth: true
          Layout.minimumWidth: 0
          text: root.win ? root.win.prop("hermes-bridge-token", "") : ""
          readOnly: true
        }
        C.SettingsButton {
          compact: true
          text: root.service ? root.service.tr("Generate", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "hermes-bridge-regen-token", [])
        }
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Save", []) : ""
        onClicked: {
          if (!root.win || !root.service) return
          root.service.client.call(root.win.winId, "hermes-bridge-host-save", [root.win.prop("hermes-bridge-host", "127.0.0.1")])
          root.service.client.call(root.win.winId, "hermes-bridge-port-save", [root.win.prop("hermes-bridge-port", "8654")])
        }
      }
    }
    C.SettingsCard {
      title: root.service ? root.service.tr("Hermes — API (suflyor asks the agent)", []) : ""
      C.SettingsCaption {
        text: root.service ? root.service.tr("Hermes API URL:", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsLineEdit {
        placeholderText: root.service ? root.service.tr("http://127.0.0.1:8642/v1", []) : ""
        text: root.win ? root.win.prop("hermes-api-url", "") : ""
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "hermes-api-url", text)
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Hermes API key (API_SERVER_KEY):", []) : ""
        font.pixelSize: Style.font.body
      }
      C.SettingsLineEdit {
        placeholderText: root.service ? root.service.tr("paste API_SERVER_KEY then Save", []) : ""
        text: root.win ? root.win.prop("hermes-api-key", "") : ""
        password: true
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "hermes-api-key", text)
      }
      C.SettingsButton {
        text: root.service ? root.service.tr("Save", []) : ""
        onClicked: {
          if (!root.win || !root.service) return
          root.service.client.call(root.win.winId, "hermes-api-url-save", [root.win.prop("hermes-api-url", "")])
          root.service.client.call(root.win.winId, "hermes-api-key-save", [root.win.prop("hermes-api-key", "")])
        }
      }
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("What you get after setup", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Ask Hermes to search calls, suggest memories and manage profiles. Prepare a call profile below to guide live AI answers.", []) : ""
      font.pixelSize: Style.font.body
    }
  }
  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Prepare a call profile", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Describe the call; the agent builds an active profile from it.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsFieldRow {
      C.SettingsLineEdit {
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        placeholderText: root.service ? root.service.tr("e.g. Interview at Acme, Senior SRE role", []) : ""
        text: root.win ? root.win.prop("hermes-profile-seed", "") : ""
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "hermes-profile-seed", text)
      }
      C.SettingsButton {
        compact: true
        text: root.service ? root.service.tr("Prepare", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "hermes-prepare-profile", [root.win.prop("hermes-profile-seed", "")])
      }
    }
    C.StatusLine {
      text: root.win ? root.win.prop("hermes-profile-status", "") : ""
      font.pixelSize: Style.font.body
    }
  }
}
