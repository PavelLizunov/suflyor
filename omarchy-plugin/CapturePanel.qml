pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons
import qs.Commons as Commons
import qs.Ui as Ui

// A card-sized protected surface plus input-only dismissal layers. Hyprland
// masks a layer's complete bounds, not its painted pixels or input region.
PanelWindow {
  id: root
  required property Item anchorItem
  required property QtObject bar
  property var owner: null
  property bool open: false
  property Item focusTarget: null
  property bool captureReady: true
  property bool concealed: false
  property int contentWidth: Style.space(440)
  property int contentHeight: Style.space(400)
  property int padding: Style.spacing.popupPadding
  property int margin: Style.gapsOut
  property var borderSpec: Border.surfaceSpec("popups", "border", Commons.Color.popups.border, Math.max(1, Style.space(2)))
  default property alias contentItem: holder.children

  readonly property var anchorWindow: anchorItem ? anchorItem.QsWindow.window : null
  readonly property string barPos: bar ? bar.position : "top"
  readonly property real screenW: screen ? screen.width : 0
  readonly property real screenH: screen ? screen.height : 0
  readonly property real barThickness: !bar ? 0
    : (typeof bar.barSizeFor === "function" && screen ? bar.barSizeFor(screen.name) : bar.barSize)
  readonly property int insetY: padding * 2 + Border.top(borderSpec) + Border.bottom(borderSpec)
  readonly property int availableW: Math.max(1, screenW - ((barPos === "left" || barPos === "right") ? barThickness + margin * 2 : margin * 2))
  readonly property int availableH: Math.max(1, screenH - ((barPos === "top" || barPos === "bottom") ? barThickness + margin * 2 : margin * 2))
  readonly property bool shown: open && captureReady && !concealed && !!anchorWindow && anchorWindow.backingWindowVisible
  readonly property var coordinator: owner || root

  function fittedContentWidth(value) { return Math.round(Math.min(Math.max(1, value), availableW)) }
  function fittedContentHeight(value) { return Math.ceil(Math.min(Math.max(insetY, value + insetY), availableH)) }
  function close() { if (owner && typeof owner.close === "function") owner.close(); else open = false }
  TransformWatcher { id: watcher; a: root.anchorWindow ? root.anchorWindow.contentItem : null; b: root.anchorItem }
  readonly property point anchorPosition: {
    watcher.transform
    return anchorWindow && anchorItem ? anchorItem.mapToItem(anchorWindow.contentItem, 0, 0) : Qt.point(0, 0)
  }
  readonly property point cardPosition: {
    var x = anchorPosition.x + (anchorItem ? anchorItem.width : 0) / 2 - contentWidth / 2
    var y = barThickness + margin
    if (barPos === "bottom") y = screenH - barThickness - contentHeight - margin
    else if (barPos === "left" || barPos === "right") {
      x = barPos === "left" ? barThickness + margin : screenW - barThickness - contentWidth - margin
      y = anchorPosition.y + (anchorItem ? anchorItem.height : 0) / 2 - contentHeight / 2
    }
    return Qt.point(Math.round(Math.max(margin, Math.min(x, screenW - contentWidth - margin))),
                    Math.round(Math.max(margin, Math.min(y, screenH - contentHeight - margin))))
  }

  visible: shown
  screen: anchorWindow ? anchorWindow.screen : null
  color: "transparent"
  exclusionMode: ExclusionMode.Ignore
  anchors { top: true; left: true }
  margins.left: cardPosition.x
  margins.top: cardPosition.y
  implicitWidth: contentWidth
  implicitHeight: contentHeight
  WlrLayershell.namespace: "suflyor-bar-panel"
  WlrLayershell.layer: WlrLayer.Overlay
  WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
  onOpenChanged: {
    if (!bar) return
    if (open) bar.requestPopout(coordinator)
    else if (bar.activePopout === coordinator) bar.releasePopout(coordinator)
  }
  onShownChanged: if (shown && focusTarget) Qt.callLater(function() { if (root.shown) root.focusTarget.forceActiveFocus() })
  Component.onDestruction: if (bar && bar.activePopout === coordinator) bar.releasePopout(coordinator)

  Ui.BorderSurface {
    anchors.fill: parent
    color: Commons.Color.popups.background
    borderSpec: root.borderSpec
    radius: Style.cornerRadius
    Item {
      id: holder
      x: Border.left(root.borderSpec) + root.padding
      y: Border.top(root.borderSpec) + root.padding
      width: Math.max(1, root.contentWidth - root.padding * 2 - Border.left(root.borderSpec) - Border.right(root.borderSpec))
      height: Math.max(1, root.contentHeight - root.insetY)
      LayoutMirroring.enabled: Commons.I18n.rightToLeft
      LayoutMirroring.childrenInherit: true
    }
  }

  Variants {
    model: root.shown ? Quickshell.screens : []
    delegate: PanelWindow {
      required property var modelData
      screen: modelData
      visible: root.shown
      color: "transparent"
      exclusionMode: ExclusionMode.Ignore
      anchors { top: true; bottom: true; left: true; right: true }
      WlrLayershell.namespace: "suflyor-dismiss"
      WlrLayershell.layer: WlrLayer.Top
      WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
      MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        onClicked: function(mouse) {
          var same = root.screen && modelData.name === root.screen.name
          var edge = root.barThickness + root.margin
          var inBar = same && (root.barPos === "bottom" ? mouse.y >= height - edge
            : root.barPos === "left" ? mouse.x <= edge
            : root.barPos === "right" ? mouse.x >= width - edge : mouse.y <= edge)
          if (inBar && root.bar && root.anchorWindow && root.bar.clickTargets) {
            var targets = root.bar.clickTargets
            for (var i = targets.length - 1; i >= 0; i--) {
              var item = targets[i]
              if (!item || !item.triggerPress || !item.visible || item.opacity === 0) continue
              if (root.bar.targetBelongsToWindow && !root.bar.targetBelongsToWindow(item, root.anchorWindow)) continue
              var point = item.mapFromItem(root.anchorWindow.contentItem, mouse.x, mouse.y)
              if (point.x >= 0 && point.y >= 0 && point.x < item.width && point.y < item.height) {
                item.triggerPress(mouse.button)
                return
              }
            }
          }
          root.close()
        }
      }
    }
  }
}
