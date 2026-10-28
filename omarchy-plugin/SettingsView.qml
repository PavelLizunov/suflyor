pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons as Commons
import qs.Ui as Ui
import "icons" as Icons
import "settings/controls" as C

// The frame owns stable geometry; pages keep their own symmetric 20px inset.
Item {
  id: root
  property var win: null
  property var service: null
  readonly property int activeTab: win ? win.prop("active-tab", 10) : 10
  readonly property bool macos: false
  readonly property int pageInset: Commons.Style.space(20)
  readonly property int scrollGutter: Commons.Style.space(14)
  readonly property bool builtinBusy: win ? win.prop("local-ai-installing", false)
    || win.prop("quality-downloading", false) || win.prop("model-switching", false)
    || win.prop("vision12b-downloading", false) || win.prop("engine-updating", false)
    || win.prop("model-speed-running", false) : false
  readonly property bool strataBusy: win ? win.prop("strata-busy", false) : false
  readonly property string installStatus: win ? win.prop("local-ai-status", "") : ""
  readonly property string qualityStatus: win ? win.prop("quality-status", "") : ""
  readonly property string visionStatus: win ? win.prop("vision12b-status", "") : ""
  readonly property string engineStatus: win ? win.prop("engine-update-status", "") : ""
  readonly property string strataStatus: win ? win.prop("strata-status", "") : ""
  property var initializedWin: null

  function relayBackgroundStatus(status) {
    if (root.win && root.service && root.initializedWin === root.win && root.activeTab !== 11)
      root.service.client.set(root.win.winId, "local-choice-status", status)
  }
  function refreshBackgroundWhenIdle() {
    if (root.win && root.service && root.initializedWin === root.win && root.activeTab !== 11
        && !root.builtinBusy && !root.strataBusy)
      root.service.client.call(root.win.winId, "now-refresh-requested", [])
  }
  onInstallStatusChanged: Qt.callLater(root.relayBackgroundStatus, root.installStatus)
  onQualityStatusChanged: Qt.callLater(root.relayBackgroundStatus, root.qualityStatus)
  onVisionStatusChanged: Qt.callLater(root.relayBackgroundStatus, root.visionStatus)
  onEngineStatusChanged: Qt.callLater(root.relayBackgroundStatus, root.engineStatus)
  onStrataStatusChanged: Qt.callLater(root.relayBackgroundStatus, root.strataStatus)
  onBuiltinBusyChanged: root.refreshBackgroundWhenIdle()
  onStrataBusyChanged: root.refreshBackgroundWhenIdle()
  onWinChanged: {
    root.initializedWin = null
    Qt.callLater(function() { root.initializedWin = root.win })
  }

  function tr(msgid, args) { return root.service ? root.service.tr(msgid, args) : msgid }
  readonly property var nav: [
    ["SESSION", 10, "Profile + context", "TabProfile", "user"],
    ["", 1, "Coaching", "TabCoaching", "coach"],
    ["AI", 15, "Components", "TabComponents", "update"],
    ["", 11, "AI providers", "TabAiProviders", "ai"],
    ["", 17, "Screenshots (F8)", "TabScreenshots", "camera"],
    ["", 12, "STT", "TabStt", "stt"],
    ["", 5, "TTS", "TabTts", "voice"],
    ["", 7, "Auto-tiles", "TabAutoTiles", "tiles"],
    ["", 9, "Knowledge base", "TabKnowledge", "knowledge"],
    ["", 16, "Hermes", "TabHermes", "brain"],
    ["APP", 6, "Audio", "TabAudio", "audio"],
    ["", 3, "Interface", "TabInterface", "monitor"],
    ["", 2, "Hotkeys", "TabHotkeys", "gear"],
    ["", 0, "Stealth", "TabStealth", "stealth"],
    ["", 4, "Updates", "TabUpdates", "update"],
    ["", 13, "Diagnostics", "TabDiagnostics", "diag"],
    ["", 14, "Memory", "TabMemory", "brain"],
  ]
  function page(tab) {
    for (var i = 0; i < nav.length; i++) if (nav[i][1] === tab) return nav[i]
    return nav[0]
  }
  function choose(tab) {
    if (!win || !service) return
    service.client.set(win.winId, "active-tab", tab)
    service.client.call(win.winId, "tab-selected", [tab])
  }
  function dismiss() {
    if (win && service) service.client.call(win.winId, "close-clicked", [])
  }

  Ui.BorderSurface {
    id: frame
    objectName: "settings-frame"
    anchors.fill: parent
    color: Commons.Color.popups.background
    radius: Commons.Style.cornerRadius
    borderSpec: Commons.Border.surfaceSpec("popups", "border", Commons.Color.popups.border, 1)
  }
  ColumnLayout {
    anchors.fill: parent
    anchors.leftMargin: frame.borderLeft + Commons.Style.spacing.popupPadding
    anchors.rightMargin: frame.borderRight + Commons.Style.spacing.popupPadding
    anchors.topMargin: frame.borderTop + Commons.Style.spacing.popupPadding
    anchors.bottomMargin: frame.borderBottom + Commons.Style.spacing.popupPadding
    spacing: Commons.Style.spacing.lg

    RowLayout {
      objectName: "settings-heading"
      Layout.fillWidth: true
      Layout.minimumHeight: Commons.Style.space(36)
      spacing: Commons.Style.spacing.controlGap
      Icons.SuflyorMark {
        objectName: "settings-brand"
        Layout.preferredWidth: Commons.Style.space(28)
        Layout.preferredHeight: Commons.Style.space(28)
      }
      DragGrip {
        Layout.preferredWidth: implicitWidth
        Layout.preferredHeight: implicitHeight
        win: root.win
        service: root.service
      }
      Text {
        Layout.fillWidth: true
        text: "Suflyor · " + root.tr("Settings")
        textFormat: Text.PlainText
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.heading
        font.bold: true
        color: Commons.Color.popups.text
        WindowDrag { anchors.fill: parent; win: root.win; service: root.service }
      }
      C.SettingsButton {
        objectName: "settings-close"
        compact: true
        iconName: "x"
        text: root.tr("Close")
        bordered: false
        onClicked: root.dismiss()
      }
    }
    Ui.PanelSeparator { Layout.fillWidth: true }
    RowLayout {
      Layout.fillWidth: true
      Layout.fillHeight: true
      spacing: Commons.Style.spacing.lg
      ScrollView {
        id: navigation
        objectName: "settings-navigation"
        Layout.preferredWidth: Commons.Style.space(202)
        Layout.fillHeight: true
        clip: true
        contentWidth: availableWidth
        rightPadding: root.scrollGutter
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        Column {
          width: navigation.availableWidth
          spacing: Commons.Style.spacing.xxs
          Repeater {
            model: root.nav
            delegate: Column {
              id: entry
              required property var modelData
              width: navigation.availableWidth
              spacing: Commons.Style.spacing.xxs
              Text {
                visible: entry.modelData[0] !== ""
                width: parent.width
                topPadding: Commons.Style.spacing.lg
                bottomPadding: Commons.Style.spacing.sm
                leftPadding: Commons.Style.spacing.lg
                text: entry.modelData[0] === "AI" ? "AI" : root.tr(entry.modelData[0])
                textFormat: Text.PlainText
                color: Commons.Color.muted
                font.family: Commons.Style.font.family
                font.pixelSize: Commons.Style.font.caption
              }
              C.SettingsButton {
                objectName: "settings-tab-" + entry.modelData[1]
                width: parent.width
                height: Math.max(implicitHeight, Commons.Style.space(28))
                iconName: entry.modelData[4]
                text: root.tr(entry.modelData[2])
                leftAlign: true
                selected: root.activeTab === entry.modelData[1]
                bordered: false
                foreground: selected ? Commons.Color.accent : Commons.Color.popups.text
                onClicked: root.choose(entry.modelData[1])
                Accessible.role: Accessible.PageTab
                Accessible.checkable: true
                Accessible.checked: selected
              }
            }
          }
        }
      }
      Ui.PanelSeparator { Layout.fillHeight: true; Layout.preferredWidth: 1 }
      ColumnLayout {
        objectName: "settings-content"
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumWidth: 0
        spacing: Commons.Style.spacing.lg
        Text {
          objectName: "settings-page-title"
          Layout.fillWidth: true
          Layout.leftMargin: root.pageInset
          Layout.rightMargin: root.pageInset + root.scrollGutter
          Layout.topMargin: Commons.Style.spacing.lg
          text: root.tr(root.page(root.activeTab)[2])
          textFormat: Text.PlainText
          font.family: Commons.Style.font.family
          font.pixelSize: Commons.Style.font.title
          font.bold: true
          color: Commons.Color.popups.text
          wrapMode: Text.Wrap
        }
        ScrollView {
          id: scroll
          objectName: "settings-page-viewport"
          Layout.fillWidth: true
          Layout.fillHeight: true
          Layout.minimumWidth: 0
          clip: true
          contentWidth: availableWidth
          rightPadding: root.scrollGutter
          ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
          Loader {
            id: pageLoader
            objectName: "settings-page"
            width: scroll.availableWidth
            source: Qt.resolvedUrl("settings/" + root.page(root.activeTab)[3] + ".qml")
            onLoaded: {
              item.win = Qt.binding(function() { return root.win })
              item.service = Qt.binding(function() { return root.service })
              const flickable = scroll.contentItem as Flickable
              if (flickable) flickable.contentY = 0
            }
          }
        }
      }
    }
  }
}
