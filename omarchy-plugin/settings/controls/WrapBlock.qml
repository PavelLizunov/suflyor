pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons

// Properties: spacing; default children stack at their actual width-aware heights.
// Use visible for conditional blocks; hidden children reserve no column space.
Column {
  width: parent ? parent.width : implicitWidth
  spacing: Style.space(8)
  height: implicitHeight
}
