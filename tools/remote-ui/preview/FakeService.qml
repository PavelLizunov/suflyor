import QtQuick
import Quickshell.Io
import "../../../omarchy-plugin"

// Inert stand-in for the plugin Service in offscreen previews: same tr() and client
// surface, no daemon, no socket. Calls are recorded, not sent.
QtObject {
  id: service

  property string locale: "ru"
  property var catalog: ({})
  property var calls: []
  property bool running: true
  // type -> RemoteWindow the fixture provides.
  property var windows: ({})

  function tr(msgid, args) {
    var text = service.locale === "ru" && service.catalog[msgid] ? service.catalog[msgid] : msgid
    if (!args) return text
    var next = 0
    return text.replace(/\{(\d*)\}/g, function (_, index) {
      var i = index === "" ? next++ : Number(index)
      return i < args.length ? String(args[i]) : ""
    })
  }

  property QtObject client: QtObject {
    property bool concealed: false
    property bool connected: true
    property bool ready: true
    signal invoked(var win, string name, var args)
    function call(win, name, args, done) { service.calls = service.calls.concat([[win, name, args]]); if (done) done(null, "") }
    function set(win, name, value) {}
    function reportGeometry(win, geometry) {}
    function first(type) { return service.windows[type] || null }
    function byType(type) { return service.windows[type] ? [service.windows[type]] : [] }
  }

  property FileView slintCatalog: FileView {
    path: Qt.resolvedUrl("../../../omarchy-plugin/locales/ru.json").toString().replace("file://", "")
    blockLoading: true
    onLoaded: service.catalog = Object.assign({}, service.catalog, JSON.parse(text()))
  }
  property FileView pluginCatalog: FileView {
    path: Qt.resolvedUrl("../../../omarchy-plugin/locales/plugin.ru.json").toString().replace("file://", "")
    blockLoading: true
    onLoaded: service.catalog = Object.assign({}, service.catalog, JSON.parse(text()))
  }
}
