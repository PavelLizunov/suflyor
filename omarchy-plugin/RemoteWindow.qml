import QtQuick

// The shell's mirror of one suflyord window. Views read `p.<name>` through `props`
// and depend on `rev`, so a property change re-evaluates only bindings of this window.
QtObject {
  id: win

  property string winId: ""
  property string type: ""
  property bool visible: false
  property var geometry: null
  property var props: ({})
  property int rev: 0

  // suflyord asked for keyboard focus (follow-up input, palette search).
  signal focusRequested()
  signal interacted()

  function prop(name, fallback) {
    var _ = win.rev
    var value = win.props[name]
    return value === undefined ? fallback : value
  }

  function update(name, value) {
    if (win.props[name] === value) return
    win.props[name] = value
    win.rev++
  }

  function replace(props, visible, geometry) {
    win.props = props
    win.visible = visible
    if (geometry) win.geometry = geometry
    win.rev++
  }
}
