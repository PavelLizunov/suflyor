import QtQuick
import Quickshell.Io
import "../../../omarchy-plugin"

// Offscreen preview of Settings: the real SettingsView over an inert service, fed with
// the SettingsWindow state suflyord sent in a headless smoke (path in `snapshotPath`).
Item {
  id: root
  width: 900
  height: 640
  property string locale: "ru"
  property string scene: "10"
  property string snapshotPath: ""
  property bool loaded: false
  // Ready once the data is in and switch/hover animations have finished.
  property bool ready: settled.fired
  Timer { id: settled; property bool fired: false; interval: 700; running: loaded && (service.catalog["Settings"] !== undefined || locale !== "ru"); onTriggered: fired = true }

  FakeService { id: service; locale: root.locale }

  RemoteWindow { id: win; winId: "SettingsWindow-5"; type: "SettingsWindow"; visible: true }

  FileView {
    path: root.snapshotPath
    blockLoading: true
    onLoaded: {
      var props = JSON.parse(text())
      props["active-tab"] = Number(root.scene)
      win.replace(props, true, null)
      root.loaded = true
    }
  }

  SettingsView { anchors.fill: parent; win: win; service: service }
}
