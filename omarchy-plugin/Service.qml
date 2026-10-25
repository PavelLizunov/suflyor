import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons as Commons

// Headless owner of suflyord and of the window mirror every Suflyor view reads.
// Starts the daemon like the telebar service starts omagramd: with a cleared
// environment, restarted after an exit, never twice.
Item {
  id: service

  property var manifest: null
  // This file sits at the plugin root, next to bin/ and locales/.
  readonly property string pluginDir: decodeURIComponent(String(Qt.resolvedUrl(".")).replace("file://", "")).replace(/\/$/, "")
  readonly property string binDir: service.pluginDir + "/bin/"

  // The interface language of the shell until suflyord reports its own setting.
  readonly property string locale: String(Commons.I18n.language || "en").indexOf("ru") === 0 ? "ru" : "en"
  // Strings of the Slint catalog (locales/ru.json) plus the plugin's own (plugin.ru.json).
  property var slintCatalog: ({})
  property var pluginCatalog: ({})
  readonly property var catalog: Object.assign({}, service.slintCatalog, service.pluginCatalog)

  readonly property alias client: remoteClient
  readonly property bool running: remoteClient.connected && remoteClient.ready
  // Three starts without a connection: the binary is missing or broken.
  property bool daemonFailed: false
  property int startAttempts: 0
  property double lastStart: 0

  // English msgid in, the language's text out; `{}` and `{0}` holes as in Slint's @tr.
  function tr(msgid, args) {
    var text = service.locale === "ru" && service.catalog[msgid] ? service.catalog[msgid] : msgid
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
    if (remoteClient.connected || service.daemonFailed) return
    var now = Date.now()
    if (now - service.lastStart < 10000) return
    if (service.startAttempts >= 3) {
      service.daemonFailed = true
      return
    }
    service.lastStart = now
    service.startAttempts++
    Quickshell.execDetached([service.binDir + "suflyord"])
  }

  RemoteClient {
    id: remoteClient
    locale: service.locale
    onConnectedChanged: if (connected) service.startAttempts = 0
  }

  Timer {
    interval: 2000
    repeat: true
    running: !remoteClient.connected && !service.daemonFailed
    onTriggered: service.ensureDaemon()
  }

  FileView {
    path: service.pluginDir + "/locales/ru.json"
    printErrors: false
    onLoaded: {
      try { service.slintCatalog = JSON.parse(text()) } catch (e) { service.slintCatalog = ({}) }
    }
  }

  FileView {
    path: service.pluginDir + "/locales/plugin.ru.json"
    printErrors: false
    onLoaded: {
      try { service.pluginCatalog = JSON.parse(text()) } catch (e) { service.pluginCatalog = ({}) }
    }
  }
}
