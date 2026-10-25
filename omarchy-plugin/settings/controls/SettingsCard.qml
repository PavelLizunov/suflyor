pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui

// Properties: title, padding, spacing; default children form a full-width column.
Ui.BorderSurface {
  id: root
  property string title: ""
  padding: Style.space(12)
  property real spacing: Style.space(8)
  default property alias content: contentColumn.data
  width: parent ? parent.width : implicitWidth
  implicitWidth: Style.space(400)
  implicitHeight: contentColumn.implicitHeight + padding * 2
  height: implicitHeight
  radius: Style.cornerRadius
  color: Style.normalFillFor(Commons.Color.foreground, Commons.Color.accent)
  borderSpec: Commons.Border.controlSpec("normal", Commons.Color.foreground, Commons.Color.accent)

  property Column body: Column {
    id: contentColumn
    parent: root
    x: root.padding
    y: root.padding
    width: Math.max(0, root.width - root.padding * 2)
    spacing: root.spacing

    SectionTitle {
      visible: root.title !== ""
      text: root.title
    }
  }
}
