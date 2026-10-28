pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons

// Native button chrome and input with the original product SVG content.
Ui.Button {
  id: root
  property string iconName: ""
  property bool compact: false
  property bool wrapLabel: false
  readonly property string projectName: iconName || icon.names[root.iconText] || ""
  readonly property bool hasIcon: projectName !== "" || iconText !== ""
  readonly property real markSize: Commons.Style.space(16)
  readonly property bool layoutManaged: parent instanceof RowLayout || parent instanceof ColumnLayout || parent instanceof GridLayout
  implicitWidth: label.implicitWidth + (hasIcon ? markSize + Commons.Style.spacing.controlGap : 0)
    + horizontalPadding * 2 + _reservedBorderLeft + _reservedBorderRight
  implicitHeight: Math.max(label.implicitHeight, markSize) + verticalPadding * 2
    + _reservedBorderTop + _reservedBorderBottom
  width: layoutManaged || compact ? implicitWidth : parent ? parent.width : implicitWidth
  height: Math.max(Commons.Style.spacing.controlHeight, implicitHeight)

  // Ui.Button does not expose an image slot. Hide only its presentation row;
  // retain its native border, pointer, keyboard and tooltip implementations.
  Component.onCompleted: {
    for (var i = 0; i < root.children.length; i++) {
      if (root.children[i] instanceof Row && root.children[i] !== content)
        root.children[i].visible = false
    }
  }

  Row {
    id: content
    objectName: root.objectName ? root.objectName + "-content" : ""
    anchors.verticalCenter: parent.verticalCenter
    x: root.leftAlign ? root._reservedContentLeftInset : Math.round((root.width - width) / 2)
    spacing: Commons.Style.spacing.controlGap
    ProjectIcon {
      id: icon
      visible: root.projectName !== ""
      name: root.projectName
      color: root.selected ? root._selectedColor : root.foreground
      width: root.markSize
      height: width
      anchors.verticalCenter: parent.verticalCenter
      rotation: root.iconRotation
      RotationAnimation on rotation {
        running: root.iconSpinning && !Commons.Style.reduceMotion
        loops: Animation.Infinite
        from: 0
        to: 360
        duration: 900
      }
    }
    Text {
      visible: root.projectName === "" && root.iconText !== ""
      text: root.iconText
      textFormat: Text.PlainText
      width: root.markSize
      horizontalAlignment: Text.AlignHCenter
      font.family: root.fontFamily
      font.pixelSize: root.iconSize
      color: root.selected ? root._selectedColor : root.foreground
      anchors.verticalCenter: parent.verticalCenter
    }
    Text {
      id: label
      visible: root.text !== ""
      text: root.text
      textFormat: Text.PlainText
      color: root.selected ? root._selectedColor : root.foreground
      font.family: root.fontFamily
      font.pixelSize: root.fontSize
      font.bold: root.selected
      wrapMode: root.wrapLabel ? Text.Wrap : Text.NoWrap
      horizontalAlignment: root.leftAlign ? Text.AlignLeft : Text.AlignHCenter
      elide: root.wrapLabel ? Text.ElideNone : Text.ElideRight
      width: Math.max(0, Math.min(implicitWidth, root.width - root.horizontalPadding * 2
        - root._reservedBorderLeft - root._reservedBorderRight
        - (root.hasIcon ? root.markSize + content.spacing : 0)))
      anchors.verticalCenter: parent.verticalCenter
    }
  }
}
