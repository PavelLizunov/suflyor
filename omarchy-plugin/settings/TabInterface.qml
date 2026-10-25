pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import "controls" as C

// SettingsWindow tab 3. Slint scheme choice is retained; the QML chrome follows Omarchy.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property var monitors: win ? win.prop("tile-monitors", []) : []
  readonly property real opacityValue: win ? win.prop("tile-body-opacity", 1.0) : 1.0
  readonly property bool macos: {
    if (!service || !service.client) return false
    var revision = service.client.globalsRev
    return (service.client.globals || {})["Platform.is-macos"] === true
  }
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(12)

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    C.SettingsRow {
      label: root.service ? root.service.tr("Language", []) : ""
      C.SettingsComboBox {
        // Language autonyms are data labels in Slint, not catalog msgids.
        model: root.service ? [root.service.tr("Русский", []), root.service.tr("English", [])] : []
        currentIndex: root.win ? root.win.prop("ui-language-index", 0) : 0
        onSelected: function(index) {
          if (root.win && root.service) root.service.client.call(root.win.winId, "language-selected", [index])
        }
      }
    }
    C.SettingsRow {
      visible: root.monitors.length > 1
      label: root.service ? root.service.tr("Tile placement", []) : ""
      C.SettingsComboBox {
        model: root.monitors
        currentIndex: root.win ? root.win.prop("tile-monitor-index", 0) : 0
        onSelected: function(index) {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "tile-monitor-index", index)
          root.service.client.call(root.win.winId, "tile-monitor-changed", [index])
        }
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Which monitor new answer tiles appear on.", []) : ""
      }
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Tile opacity", []) : ""
      C.SettingsSlider {
        minimum: 0.5
        maximum: 1.0
        value: root.opacityValue
        onChanged: function(value) {
          if (!root.win || !root.service) return
          root.service.client.set(root.win.winId, "tile-body-opacity", value)
          root.service.client.call(root.win.winId, "tile-body-opacity-changed", [value])
        }
      }
      C.StatusLine {
        // Like Slint: numeric data followed by a percent sign, not a new msgid.
        text: Math.round(root.opacityValue * 100) + "%"
      }
      C.SettingsCaption {
        text: root.service ? root.service.tr("Lower values let you see the meeting through tiles. 100% = opaque, 50% = half-transparent.", []) : ""
      }
    }
    C.SettingsRow {
      label: root.service ? root.service.tr("Colour scheme", []) : ""
      C.SettingsComboBox {
        // Proper names are literal model labels in Slint, not catalog msgids.
        model: root.service ? [root.service.tr("Glacier", []), root.service.tr("Graphite", []), root.service.tr("Obsidian", []), root.service.tr("Light Frost", [])] : []
        currentIndex: root.win ? root.win.prop("color-scheme-index", 0) : 0
        onSelected: function(index) {
          if (root.win && root.service) root.service.client.call(root.win.winId, "color-scheme-selected", [index])
        }
      }
    }
    C.SettingsRow {
      visible: !root.macos
      label: root.service ? root.service.tr("Setup", []) : ""
      C.SettingsButton {
        text: root.service ? root.service.tr("Run setup wizard", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "open-wizard-clicked", [])
      }
    }
  }
}
