pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import "small" as Small
import qs.Commons as Commons

Item {
  id: root
  property var win
  property var service
  function p(name, fallback) { return win ? win.prop(name, fallback) : fallback }
  function set(name, value) { if (win && service) service.client.set(win.winId, name, value) }
  function call(name, args) { if (win && service) service.client.call(win.winId, name, args || []) }
  function tr(text, args) { return service ? service.tr(text, args || []) : "" }
  function submit() {
    if (p("query", "") === "") return
    call("submitted", [p("query", "")])
    set("query", "")
  }
  Accessible.name: root.tr("Suflyor — question")
  // drag-start-requested / drag-moved are native-window chrome, owned by AuxLayer.
  Small.Surface {
    anchors.fill: parent
    view: root
    closeCallback: "cancelled"
    title: root.tr("Write a question for the assistant")
    note: root.tr("{}", [root.p("active-profile", "")])
    focusTarget: input
    RowLayout {
      anchors.fill: parent
      spacing: Commons.Style.spacing.sm
      Small.Input {
        id: input
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        Layout.alignment: Qt.AlignTop
        placeholderText: root.tr("Type your question…")
        text: root.p("query", "")
        onTextEdited: root.set("query", text)
        onAccepted: root.submit()
      }
      Small.Action {
        Layout.fillWidth: false
        Layout.preferredWidth: implicitWidth
        Layout.alignment: Qt.AlignTop
        iconText: "\uf1d8"
        text: root.tr("Send")
        enabled: root.p("query", "") !== ""
        selected: true
        onClicked: root.submit()
      }
    }
  }
}
