import QtQuick
import qs.Commons as Commons
import "../../../omarchy-plugin" as Plugin

// Actual Suflyor bar widgets with inert state; the neighbouring Tera icon is a size reference.
Rectangle {
  id: root
  width: 280
  height: 64
  color: Commons.Color.bar.background
  property bool verticalBar: false
  property string referenceIconPath: ""
  property bool ready: settled.fired

  function previewService(streaming) {
    var win = {
      winId: streaming ? "preview-ai" : "preview-idle",
      prop: function(name, fallback) {
        if (name === "ai-streaming") return streaming
        if (name === "status-text") return "idle"
        return fallback
      }
    }
    return {
      running: true,
      daemonFailed: false,
      tr: function(msgid, args) { return msgid },
      client: {
        concealed: false,
        first: function(type) { return type === "OverlayBarWindow" ? win : null }
      }
    }
  }

  function imageReady(item) {
    if (item instanceof Image) return item.status === Image.Ready
    for (var i = 0; i < item.children.length; i++) {
      if (imageReady(item.children[i])) return true
    }
    return false
  }

  QtObject {
    id: barModel
    property bool vertical: root.verticalBar
    property int barSize: vertical ? Commons.Style.bar.sizeVertical : Commons.Style.bar.sizeHorizontal
    property string fontFamily: Commons.Style.font.family
    property color barForeground: Commons.Color.bar.text
    property color urgent: Commons.Color.urgent
    property bool foregroundAnimationEnabled: false
  }

  Timer {
    id: settled
    property bool fired: false
    interval: 50
    repeat: true
    running: !fired
    onTriggered: {
      if (root.imageReady(idleWidget) && root.imageReady(activeWidget)
          && (!root.referenceIconPath || reference.status === Loader.Ready))
        fired = true
    }
  }

  Row {
    x: 20
    y: 4
    spacing: Commons.Style.space(24)

    Column {
      visible: root.referenceIconPath !== ""
      spacing: 4
      Item {
        width: Commons.Style.bar.iconSlot
        height: barModel.barSize
        Loader {
          id: reference
          anchors.centerIn: parent
          source: root.referenceIconPath
          onLoaded: {
            item.iconSize = Qt.binding(function() { return Commons.Style.bar.iconCanvas })
            item.color = Qt.binding(function() { return Commons.Color.bar.text })
          }
        }
      }
      Text {
        textFormat: Text.PlainText
        text: "Tera"
        color: Commons.Color.bar.text
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.caption
        anchors.horizontalCenter: parent.horizontalCenter
      }
    }

    Column {
      spacing: 4
      Plugin.BarWidget {
        id: idleWidget
        objectName: "brand-idle-widget"
        bar: barModel
        service: root.previewService(false)
      }
      Text {
        textFormat: Text.PlainText
        text: "Suflyor"
        color: Commons.Color.bar.text
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.caption
        anchors.horizontalCenter: parent.horizontalCenter
      }
    }

    Column {
      spacing: 4
      Plugin.BarWidget {
        id: activeWidget
        objectName: "brand-active-widget"
        bar: barModel
        service: root.previewService(true)
      }
      Text {
        textFormat: Text.PlainText
        text: "AI"
        color: Commons.Color.bar.text
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.caption
        anchors.horizontalCenter: parent.horizontalCenter
      }
    }
  }
}
