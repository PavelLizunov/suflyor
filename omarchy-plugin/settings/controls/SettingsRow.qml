pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons

// Properties: label, labelWidth, spacing; default children = stretching control/captions.
Item {
  id: root
  property string label: ""
  property real labelWidth: Style.space(132)
  property real spacing: Style.space(8)
  default property alias content: controls.data
  width: parent ? parent.width : implicitWidth
  implicitHeight: Math.max(labelText.implicitHeight, controls.implicitHeight)
  height: implicitHeight

  property SettingsCaption labelItem: SettingsCaption {
    id: labelText
    parent: root
    width: Math.min(root.labelWidth, root.width * 0.35)
    topPadding: Style.space(8)
    bottomPadding: Style.space(8)
    text: root.label
    color: Commons.Color.foreground
    font.pixelSize: Style.font.body
  }
  property Column body: Column {
    id: controls
    parent: root
    x: labelText.width + root.spacing
    width: Math.max(0, root.width - x)
    spacing: Style.space(4)
  }
}
