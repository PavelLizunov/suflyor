pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons
import "controls" as C

// SettingsWindow tab 4, Linux release-page updates and full/server-only transfers.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property bool checking: win ? win.prop("update-checking", false) : false
  readonly property bool available: win ? win.prop("update-available", false) : false
  readonly property bool hasSecrets: win ? win.prop("backup-has-secrets", false) : false
  // Slint's changed active-tab requests backup status on entry. Loader supplies win/service later.
  readonly property var backupWindow: win && service ? win : null
  onBackupWindowChanged: if (backupWindow) root.service.client.call(backupWindow.winId, "backup-status-requested", [])
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Updates", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Check GitHub for a newer release. Open its release page to update the native Linux package. Your configuration and models are retained.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsButtonsRow {
      C.SettingsButton {
        text: root.service ? root.service.tr(root.checking ? "Checking…" : "Check for updates", []) : ""
        enabled: !root.checking
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "check-updates-clicked", [])
      }
      C.SettingsButton {
        visible: root.available
        text: root.service ? root.service.tr("Open release page", []) : ""
        enabled: !root.checking
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "install-update-clicked", [])
      }
    }
    C.StatusLine {
      text: root.win ? root.win.prop("update-status", "") : ""
      tone: root.available ? "good" : "muted"
      font.pixelSize: Style.font.body
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Backup and transfer", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Version: {}", [root.win ? root.win.prop("app-version", "") : ""]) : ""
    }
    C.WrapBlock {
      C.StatusLine {
        visible: root.hasSecrets
        text: root.service ? root.service.tr("Warning:", []) : ""
        tone: "warning"
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr(root.hasSecrets ? "Keys included. Keep private; import replaces settings." : "Import replaces the current settings.", []) : ""
      }
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Export settings (incl. keys)…", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "export-profile-clicked", [])
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Import settings…", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "import-profile-clicked", [])
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Server transfer copies AI/STT providers, URLs, models and legacy bridge keys. Protected OpenAI and Anthropic API keys stay in this user's local credentials file. Local UI, profiles, devices and snippets stay unchanged.", []) : ""
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Export server settings…", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "export-server-settings-clicked", [])
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Import server settings…", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "import-server-settings-clicked", [])
    }
    C.SettingsCard {
      visible: root.win ? root.win.prop("server-preview-ready", false) : false
      spacing: Style.space(4)
      C.SettingsCaption {
        text: root.service ? root.service.tr("Review changes before applying (keys shown as set/— only, never the value):", []) : ""
        color: Commons.Color.foreground
        font.pixelSize: Style.font.body
        font.bold: true
      }
      C.SettingsCaption {
        text: root.win ? root.win.prop("server-preview-cloud", "") : ""
        color: Commons.Color.foreground
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        text: root.win ? root.win.prop("server-preview-local", "") : ""
        color: Commons.Color.foreground
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        text: root.win ? root.win.prop("server-preview-vision", "") : ""
        color: Commons.Color.foreground
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        text: root.win ? root.win.prop("server-preview-stt", "") : ""
        color: Commons.Color.foreground
        font.pixelSize: Style.font.body
      }
      C.SettingsCaption {
        text: root.win ? root.win.prop("server-preview-gigaam", "") : ""
        color: Commons.Color.urgent
      }
      C.SettingsButtonsRow {
        C.SettingsButton {
          text: root.service ? root.service.tr("Apply server settings", []) : ""
          foreground: Commons.Color.accent
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "apply-server-settings-clicked", [])
        }
        C.SettingsButton {
          text: root.service ? root.service.tr("Cancel", []) : ""
          onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "cancel-server-settings-clicked", [])
        }
      }
    }
    C.StatusLine {
      text: root.win ? root.win.prop("profile-io-result", "") : ""
      tone: "accent"
      font.pixelSize: Style.font.body
    }
  }
}
