import QtQuick
import qs.Commons as Commons
import qs.Ui as Ui
import "../../../omarchy-plugin/icons" as Icons

// Original product assets inside the host's actual native bar icon consumer.
Rectangle {
  width: 200
  height: 58
  color: Commons.Color.bar.background
  property bool ready: settled.fired
  Timer {
    id: settled
    property bool fired: false
    interval: 700
    running: true
    onTriggered: fired = true
  }
  Row {
    anchors.horizontalCenter: parent.horizontalCenter
    y: 4
    spacing: Commons.Style.space(32)
    Column {
      spacing: 2
      Ui.BarIconButton {
        objectName: "brand-idle-slot"
        interactive: false
        pressable: false
        iconComponent: Component { Icons.SuflyorMark {} }
      }
      Text {
        text: "idle"
        color: Commons.Color.bar.text
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.caption
        anchors.horizontalCenter: parent.horizontalCenter
      }
    }
    Column {
      spacing: 2
      Ui.BarIconButton {
        objectName: "brand-active-slot"
        interactive: false
        pressable: false
        iconComponent: Component {
          Icons.SuflyorMark { assistantActive: true; ink: Commons.Color.accent }
        }
      }
      Text {
        text: "AI"
        color: Commons.Color.bar.text
        font.family: Commons.Style.font.family
        font.pixelSize: Commons.Style.font.caption
        anchors.horizontalCenter: parent.horizontalCenter
      }
    }
  }
}
