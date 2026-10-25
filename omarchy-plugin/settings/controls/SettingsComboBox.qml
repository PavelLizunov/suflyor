pragma ComponentBehavior: Bound
import QtQuick
import qs.Commons
import qs.Ui as Ui

// Properties: model (string array), currentIndex, enabled; signal selected(int index).
// Index values keep duplicate labels distinct. Selection stays bound to the remote mirror.
Ui.Dropdown {
  id: root
  property var model: []
  property int currentIndex: -1
  signal selected(int index)
  width: parent ? parent.width : implicitWidth
  height: implicitHeight
  showLabel: false
  options: root.model.map(function(label, index) { return {value: String(index), label: label} })
  value: currentIndex >= 0 && currentIndex < model.length ? String(currentIndex) : ""
  opacity: enabled ? 1 : 0.5
  onChanged: function(value) {
    root.selected(Number(value))
    // Ui.Dropdown writes value internally: restore the model-driven binding afterwards.
    root.value = Qt.binding(function() {
      return root.currentIndex >= 0 && root.currentIndex < root.model.length ? String(root.currentIndex) : ""
    })
  }
}
