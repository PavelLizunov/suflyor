pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
import qs.Commons as Commons
import "../../tile" as Tile

RowLayout {
  id: root
  required property var view
  spacing: Commons.Style.spacing.sm
  Tile.ActionButton {
    iconText: root.view.p("playing", false) ? "\uf04c" : "\uf04b"
    label: root.view.service ? root.view.service.tr("Pause / resume reading aloud", []) : ""
    selected: true
    onClicked: root.view.call("toggle-play")
  }
  Ui.PanelSlider {
    Layout.fillWidth: true
    Layout.minimumWidth: Commons.Style.space(60)
    value: root.view.p("progress", 0)
    trackColor: Commons.Style.normalBorderColor
    fillColor: Commons.Color.accent
    knobColor: Commons.Color.accent
    onMoved: function(value) { root.view.call("seek-fraction", [value]) }
  }
  Tile.Label {
    text: root.view.p("time-text", "")
    font.pixelSize: Commons.Style.font.caption
    color: Commons.Color.muted
  }
  Ui.Dropdown {
    Layout.preferredWidth: Commons.Style.space(90)
    showLabel: false
    options: ["1×", "1.25×", "1.5×", "1.75×", "2×", "2.25×", "2.5×", "2.75×", "3×"]
    value: options[root.view.p("speed-index", 0)] || options[0]
    onChanged: function(next) {
      var index = options.indexOf(next)
      if (index < 0) return
      root.view.set("speed-index", index)
      root.view.call("set-speed", [1 + index * 0.25])
    }
  }
  Tile.Label { text: "\uf028"; color: Commons.Color.accent }
  Ui.PanelSlider {
    Layout.preferredWidth: Commons.Style.space(90)
    minimum: 0
    maximum: 3
    value: root.view.p("volume", 1)
    trackColor: Commons.Style.normalBorderColor
    fillColor: Commons.Color.accent
    knobColor: Commons.Color.accent
    onMoved: function(value) {
      root.view.set("volume", value)
      root.view.call("set-volume", [value])
    }
  }
}
