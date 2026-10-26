pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
import qs.Commons as Commons

// Headless owner of suflyord and of the window mirror every Suflyor view reads.
// Starts the daemon like the telebar service starts omagramd: with a cleared
// environment, restarted after an exit, never twice.
Item {
  id: root

  property var manifest: null
  // This file sits at the plugin root, next to bin/ and locales/.
  readonly property string pluginDir: decodeURIComponent(String(Qt.resolvedUrl(".")).replace("file://", "")).replace(/\/$/, "")
  readonly property string binDir: root.pluginDir + "/bin/"

  // Suflyor's own interface language once suflyord reports it, the shell's before that.
  readonly property string shellLocale: String(Commons.I18n.language || "en").indexOf("ru") === 0 ? "ru" : "en"
  readonly property string locale: {
    var _ = remoteClient.globalsRev
    return remoteClient.globals["Suflyor.locale"] || root.shellLocale
  }
  // Strings of the Slint catalog (locales/ru.json) plus the plugin's own (plugin.ru.json).
  property var slintCatalog: ({})
  property var pluginCatalog: ({})
  readonly property var catalog: Object.assign({}, root.slintCatalog, root.pluginCatalog)

  readonly property alias client: remoteClient
  readonly property bool running: remoteClient.connected && remoteClient.ready
  // Three starts without a connection: the binary is missing or broken.
  property bool daemonFailed: false
  property int startAttempts: 0
  property double lastStart: 0

  // English msgid in, the language's text out; `{}` and `{0}` holes as in Slint's @tr.
  function tr(msgid, args) {
    var text = root.locale === "ru" && root.catalog[msgid] ? root.catalog[msgid] : msgid
    if (!args) return text
    var next = 0
    return text.replace(/\{(\d*)\}/g, function (_, index) {
      var i = index === "" ? next++ : Number(index)
      return i < args.length ? String(args[i]) : ""
    })
  }

  // suflyord runs detached, so a shell restart does not end a session; its singleton
  // lock makes a second start exit at once. Started only while nobody answers.
  function ensureDaemon() {
    if (remoteClient.connected || root.daemonFailed) return
    var now = Date.now()
    if (now - root.lastStart < 10000) return
    if (root.startAttempts >= 3) {
      root.daemonFailed = true
      return
    }
    root.lastStart = now
    root.startAttempts++
    Quickshell.execDetached([root.binDir + "suflyord"])
  }

  // A surface opens on the focused monitor, as the shell's own overlays do, and stays there.
  function focusedScreen() {
    var monitor = Hyprland.focusedMonitor
    var name = monitor ? String(monitor.name || "") : ""
    for (var i = 0; i < Quickshell.screens.length; i++) {
      if (Quickshell.screens[i].name === name) return Quickshell.screens[i]
    }
    return Quickshell.screens.length > 0 ? Quickshell.screens[0] : null
  }

  RemoteClient {
    id: remoteClient
    locale: root.shellLocale
    onConnectedChanged: if (connected) root.startAttempts = 0
  }

  // One layer per tile; Variants keeps a tile's surface while others open and close.
  Variants {
    model: remoteClient.windowList.filter(function (w) { return w.type === "TileWindow" })
    delegate: TileLayer { service: root }
  }

  Variants {
    model: remoteClient.windowList.filter(function (w) { return w.type === "SettingsWindow" })
    delegate: SettingsLayer { service: root }
  }

  // Shown on Omarchy by other means: the tray menu by the bar widget, the lock menu
  // inside the bar panel, region capture by slurp.
  readonly property var auxTypes: ["ArchiveWindow", "TranscriptWindow", "PaletteWindow", "HelpWindow",
    "WizardWindow", "RecoverOfferWindow", "TextAskWindow"]
  Variants {
    model: remoteClient.windowList.filter(function (w) { return root.auxTypes.indexOf(w.type) >= 0 })
    delegate: AuxLayer { service: root }
  }

  Timer {
    interval: 2000
    repeat: true
    running: !remoteClient.connected && !root.daemonFailed
    onTriggered: root.ensureDaemon()
  }

  FileView {
    path: root.pluginDir + "/locales/ru.json"
    printErrors: false
    onLoaded: {
      try { root.slintCatalog = JSON.parse(text()) } catch (e) { root.slintCatalog = ({}) }
    }
  }

  FileView {
    path: root.pluginDir + "/locales/plugin.ru.json"
    printErrors: false
    onLoaded: {
      try { root.pluginCatalog = JSON.parse(text()) } catch (e) { root.pluginCatalog = ({}) }
    }
  }
}
