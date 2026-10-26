pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import qs.Commons
import qs.Commons as Commons
import "icons" as Icons

// Suflyor Settings in the Omarchy style: grouped navigation on the left, the page of
// `active-tab` on the right. Each page is settings/Tab<Name>.qml with `win` and `service`.
// Tab numbers are SettingsWindow's own (settings_panel.slint), not the visual order.
Item {
  id: root

  property var win: null
  property var service: null
  readonly property int activeTab: win ? win.prop("active-tab", 10) : 10
  readonly property bool macos: false
  readonly property bool builtinBusy: win ? win.prop("local-ai-installing", false)
    || win.prop("quality-downloading", false) || win.prop("model-switching", false)
    || win.prop("vision12b-downloading", false) || win.prop("engine-updating", false) : false
  readonly property bool strataBusy: win ? win.prop("strata-busy", false) : false
  readonly property string installStatus: win ? win.prop("local-ai-status", "") : ""
  readonly property string qualityStatus: win ? win.prop("quality-status", "") : ""
  readonly property string visionStatus: win ? win.prop("vision12b-status", "") : ""
  readonly property string engineStatus: win ? win.prop("engine-update-status", "") : ""
  readonly property string strataStatus: win ? win.prop("strata-status", "") : ""
  property var initializedWin: null

  // TabAiProviders handles these while loaded; Slint's window-level handlers
  // must also survive navigating away from a running download or model switch.
  function relayBackgroundStatus(status) {
    if (root.win && root.service && root.initializedWin === root.win && root.activeTab !== 11)
      root.service.client.set(root.win.winId, "local-choice-status", status)
  }
  function refreshBackgroundWhenIdle() {
    if (root.win && root.service && root.initializedWin === root.win && root.activeTab !== 11
        && !root.builtinBusy && !root.strataBusy)
      root.service.client.call(root.win.winId, "now-refresh-requested", [])
  }
  onInstallStatusChanged: root.relayBackgroundStatus(root.installStatus)
  onQualityStatusChanged: root.relayBackgroundStatus(root.qualityStatus)
  onVisionStatusChanged: root.relayBackgroundStatus(root.visionStatus)
  onEngineStatusChanged: root.relayBackgroundStatus(root.engineStatus)
  onStrataStatusChanged: root.relayBackgroundStatus(root.strataStatus)
  onBuiltinBusyChanged: root.refreshBackgroundWhenIdle()
  onStrataBusyChanged: root.refreshBackgroundWhenIdle()
  onWinChanged: {
    root.initializedWin = null
    Qt.callLater(function() { root.initializedWin = root.win })
  }

  function tr(msgid, args) { return root.service ? root.service.tr(msgid, args) : msgid }

  // [group header msgid or "", tab number, label msgid, page file, Nerd Font glyph]
  readonly property var nav: [
    ["SESSION", 10, "Profile + context", "TabProfile", "\u{f0004}"],
    ["", 1, "Coaching", "TabCoaching", "\u{f0eb6}"],
    ["AI", 15, "Components", "TabComponents", "\u{f01da}"],
    ["", 11, "AI providers", "TabAiProviders", "\u{f06a9}"],
    ["", 17, "Screenshots (F8)", "TabScreenshots", "\u{f0100}"],
    ["", 12, "STT", "TabStt", "\u{f036c}"],
    ["", 5, "TTS", "TabTts", "\u{f057e}"],
    ["", 7, "Auto-tiles", "TabAutoTiles", "\u{f0570}"],
    ["", 9, "Knowledge base", "TabKnowledge", "\u{f00ba}"],
    ["", 16, "Hermes", "TabHermes", "\u{f06a9}"],
    ["APP", 6, "Audio", "TabAudio", "\u{f04c3}"],
    ["", 3, "Interface", "TabInterface", "\u{f0379}"],
    ["", 2, "Hotkeys", "TabHotkeys", "\u{f030c}"],
    ["", 0, "Stealth", "TabStealth", "\u{f0209}"],
    ["", 4, "Updates", "TabUpdates", "\u{f06b0}"],
    ["", 13, "Diagnostics", "TabDiagnostics", "\u{f0a9a}"],
    ["", 14, "Memory", "TabMemory", "\u{f0369}"],
  ]

  function page(tab) {
    for (var i = 0; i < root.nav.length; i++) if (root.nav[i][1] === tab) return root.nav[i]
    return root.nav[0]
  }

  function choose(tab) {
    if (!root.win || !root.service) return
    root.service.client.set(root.win.winId, "active-tab", tab)
    root.service.client.call(root.win.winId, "tab-selected", [tab])
  }

  Rectangle {
    anchors.fill: parent
    color: Commons.Color.background
    radius: Style.space(10)
    border.width: 1
    border.color: Qt.alpha(Commons.Color.menu.border, 0.35)
  }

  Row {
    anchors.fill: parent
    anchors.margins: 1

    // ---- navigation
    Item {
      id: sidebar
      width: Style.space(200)
      height: parent.height

      Row {
        id: brand
        x: Style.space(14)
        y: Style.space(14)
        spacing: Style.space(8)
        Icons.SuflyorMark {
          width: Style.space(18)
          height: width
          ink: Commons.Color.foreground
          anchors.verticalCenter: parent.verticalCenter
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: root.tr("Settings")
          color: Commons.Color.foreground
          font.family: Style.font.family
          font.pixelSize: Style.font.heading
          font.weight: Font.Bold
        }
      }

      ScrollView {
        anchors.top: brand.bottom
        anchors.topMargin: Style.space(12)
        anchors.bottom: closeButton.top
        anchors.bottomMargin: Style.space(8)
        width: parent.width
        clip: true

        Column {
          width: sidebar.width
          spacing: Style.space(2)

          Repeater {
            model: root.nav
            delegate: Column {
              id: entry
              required property var modelData
              width: sidebar.width

              Text {
                visible: entry.modelData[0] !== ""
                leftPadding: Style.space(16)
                topPadding: Style.space(10)
                bottomPadding: Style.space(4)
                text: entry.modelData[0] === "AI" ? "AI" : root.tr(entry.modelData[0])
                color: Commons.Color.muted
                font.family: Style.font.family
                font.pixelSize: Style.font.caption
                font.letterSpacing: 0.6
              }

              Rectangle {
                id: item
                readonly property bool active: root.activeTab === entry.modelData[1]
                x: Style.space(8)
                width: sidebar.width - Style.space(16)
                height: Style.space(30)
                radius: Style.space(6)
                color: active ? Qt.alpha(Commons.Color.accent, 0.16) : hover.hovered ? Qt.alpha(Commons.Color.foreground, 0.06) : "transparent"
                border.width: activeFocus ? 1 : 0
                border.color: Commons.Color.accent

                Rectangle {
                  visible: item.active
                  width: 3
                  height: parent.height - Style.space(10)
                  radius: 2
                  anchors.verticalCenter: parent.verticalCenter
                  color: Commons.Color.accent
                }

                Row {
                  anchors.verticalCenter: parent.verticalCenter
                  x: Style.space(12)
                  spacing: Style.space(8)
                  Text {
                    text: entry.modelData[4]
                    color: item.active ? Commons.Color.accent : Commons.Color.muted
                    font.family: Style.font.family
                    font.pixelSize: Style.font.body
                    anchors.verticalCenter: parent.verticalCenter
                  }
                  Text {
                    text: entry.modelData[1] === 4 && root.macos ? root.tr("Backup") : root.tr(entry.modelData[2])
                    color: item.active ? Commons.Color.accent : Commons.Color.foreground
                    font.family: Style.font.family
                    font.pixelSize: Style.font.body
                    anchors.verticalCenter: parent.verticalCenter
                  }
                }

                activeFocusOnTab: true
                Keys.onReturnPressed: root.choose(entry.modelData[1])
                Keys.onEnterPressed: root.choose(entry.modelData[1])
                Keys.onSpacePressed: root.choose(entry.modelData[1])
                HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: { item.forceActiveFocus(); root.choose(entry.modelData[1]) } }
                Accessible.role: Accessible.PageTab
                Accessible.name: root.tr(entry.modelData[2])
                Accessible.checkable: true
                Accessible.checked: item.active
                Accessible.onPressAction: root.choose(entry.modelData[1])
              }
            }
          }
        }
      }

      Rectangle {
        id: closeButton
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Style.space(12)
        x: Style.space(8)
        width: sidebar.width - Style.space(16)
        height: Style.space(32)
        radius: Style.space(6)
        color: closeHover.hovered ? Qt.alpha(Commons.Color.urgent, 0.13) : "transparent"
        border.width: 1
        border.color: activeFocus ? Commons.Color.accent
          : closeHover.hovered ? Qt.alpha(Commons.Color.urgent, 0.55) : Qt.alpha(Commons.Color.foreground, 0.16)
        Row {
          anchors.centerIn: parent
          spacing: Style.space(7)
          Text { text: "\u{f0156}"; color: Commons.Color.muted; font.family: Style.font.family; font.pixelSize: Style.font.body }
          Text { text: root.tr("Close"); color: Commons.Color.muted; font.family: Style.font.family; font.pixelSize: Style.font.body }
        }
        function dismiss() {
          if (root.win && root.service) root.service.client.call(root.win.winId, "close-clicked", [])
        }
        activeFocusOnTab: true
        Keys.onReturnPressed: closeButton.dismiss()
        Keys.onEnterPressed: closeButton.dismiss()
        Keys.onSpacePressed: closeButton.dismiss()
        HoverHandler { id: closeHover; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: closeButton.dismiss() }
        Accessible.role: Accessible.Button
        Accessible.name: root.tr("Close")
        Accessible.onPressAction: closeButton.dismiss()
      }
    }

    Rectangle { width: 1; height: parent.height; color: Qt.alpha(Commons.Color.foreground, 0.08) }

    // ---- page
    Item {
      id: content
      width: parent.width - sidebar.width - 1
      height: parent.height

      Column {
        id: head
        width: parent.width
        padding: Style.space(20)
        bottomPadding: Style.space(12)
        spacing: Style.space(12)
        Text {
          text: root.activeTab === 4 && root.macos ? root.tr("Backup") : root.tr(root.page(root.activeTab)[2])
          color: Commons.Color.foreground
          font.family: Style.font.family
          font.pixelSize: Style.font.heading
          font.weight: Font.Bold
        }
        Rectangle { width: head.width - 2 * head.padding; height: 1; color: Qt.alpha(Commons.Color.foreground, 0.1) }
      }

      ScrollView {
        id: scroll
        anchors.top: head.bottom
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        contentWidth: availableWidth

        Loader {
          id: pageLoader
          width: scroll.availableWidth
          source: Qt.resolvedUrl("settings/" + root.page(root.activeTab)[3] + ".qml")
          onLoaded: {
            item.win = Qt.binding(function () { return root.win })
            item.service = Qt.binding(function () { return root.service })
            // Slint resets content-scroll.viewport-y whenever active-tab changes.
            const flickable = scroll.contentItem as Flickable
            if (flickable) flickable.contentY = 0
          }
        }
      }
    }
  }
}
