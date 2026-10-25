pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import "controls" as C

// SettingsWindow tab 14. The backend owns list mutations; only edit mode/buffers change here.
Column {
  id: root
  property var win: null
  property var service: null
  readonly property var candidates: win ? win.prop("memory-candidates", []) : []
  readonly property var items: win ? win.prop("memory-items", []) : []
  readonly property int editingId: win ? win.prop("memory-editing-id", -1) : -1
  readonly property string editText: win ? win.prop("memory-edit-text", "") : ""
  height: implicitHeight
  padding: Style.space(20)
  topPadding: 0
  spacing: Style.space(10)

  // Rust sends emoji as kind identifiers. Render equivalent native glyphs, never emoji.
  function kindIcon(kind) {
    switch (kind) {
    case "\u{1f4dd}": return "\u{f0219}"
    case "\u{1f501}": return "\u{f0456}"
    case "\u2699": return "\u{f0493}"
    case "\u2b50": return "\u{f04ce}"
    case "\u{1f4cc}": return "\u{f0403}"
    default: return "\u{f0764}"
    }
  }

  C.SettingsCard {
    width: Math.max(0, root.width - root.padding * 2)
    title: root.service ? root.service.tr("Personal memory", []) : ""
    C.SettingsCaption {
      text: root.service ? root.service.tr("Personal memory is stored locally. Approved items are added to new AI requests and may be sent to your configured AI provider.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsFieldRow {
      C.SettingsLineEdit {
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        placeholderText: root.service ? root.service.tr("Your fact: project, term, names…", []) : ""
        text: root.win ? root.win.prop("memory-add-text", "") : ""
        onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "memory-add-text", text)
        onAccepted: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-add-fact", [text])
      }
      C.SettingsButton {
        compact: true
        text: root.service ? root.service.tr("Add", []) : ""
        onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-add-fact", [root.win.prop("memory-add-text", "")])
      }
    }
    C.SettingsCaption {
      text: root.service ? root.service.tr("Facts also reach the meeting Summary — but only when their terms come up in the conversation.", []) : ""
      font.pixelSize: Style.font.body
    }
    C.SettingsButton {
      text: root.service ? root.service.tr("Extract from recent sessions", []) : ""
      onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-extract", [])
    }
    C.StatusLine {
      text: root.win ? root.win.prop("memory-status", "") : ""
      tone: "accent"
      font.pixelSize: Style.font.body
    }

    C.SectionTitle {
      text: root.service ? root.service.tr("Suggestions ({})", [root.candidates.length]) : ""
    }
    C.SettingsCaption {
      visible: root.candidates.length === 0
      text: root.service ? root.service.tr("No suggestions yet — click 'Extract from recent sessions' after a session.", []) : ""
      font.pixelSize: Style.font.body
    }
    Repeater {
      model: root.candidates
      delegate: C.SettingsCard {
        id: candidate
        required property var modelData
        padding: Style.space(8)
        spacing: Style.space(4)
        clip: true
        // Bound text, not actions, to the original 120px row budget.
        RowLayout {
          id: candidateHeader
          width: parent.width
          spacing: Style.space(6)
          C.IconSlot { icon: root.kindIcon(candidate.modelData.kind) }
          C.SettingsCaption {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            text: candidate.modelData.reason
            maximumLineCount: 1
            elide: Text.ElideRight
          }
        }
        C.SettingsCaption {
          text: candidate.modelData.text
          font.pixelSize: Style.font.body
          height: Math.min(implicitHeight, Math.max(0, Style.space(120) - candidate.padding * 2 - candidate.spacing * 2 - candidateHeader.height - candidateActions.height))
          clip: true
        }
        C.SettingsButtonsRow {
          id: candidateActions
          C.SettingsButton {
            text: root.service ? root.service.tr("Approve", []) : ""
            onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-approve", [candidate.modelData.id])
          }
          C.SettingsButton {
            text: root.service ? root.service.tr("Reject", []) : ""
            onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-reject", [candidate.modelData.id])
          }
        }
      }
    }

    C.SectionTitle {
      text: root.service ? root.service.tr("Approved memory ({})", [root.items.length]) : ""
    }
    C.SettingsCaption {
      visible: root.items.length === 0
      text: root.service ? root.service.tr("No approved memory yet.", []) : ""
      font.pixelSize: Style.font.body
    }
    Repeater {
      model: root.items
      delegate: C.SettingsCard {
        id: memory
        required property var modelData
        readonly property string sourceText: modelData["source-text"] || ""
        readonly property string normStatus: modelData["norm-status"] || "none"
        readonly property bool originalDiffers: sourceText !== "" && sourceText !== modelData.text
        padding: Style.space(8)
        spacing: Style.space(4)
        clip: true
        C.WrapBlock {
          visible: root.editingId !== memory.modelData.id
          spacing: Style.space(4)
          Item {
            id: fact
            width: parent.width
            implicitHeight: Math.max(kind.height, factText.implicitHeight)
            height: Math.min(implicitHeight, Math.max(0, Style.space(120) - memory.padding * 2 - Style.space(4) - memoryActions.height))
            clip: true
            C.IconSlot {
              id: kind
              icon: root.kindIcon(memory.modelData.kind)
            }
            Column {
              id: factText
              x: kind.width + Style.space(6)
              width: Math.max(0, fact.width - x)
              spacing: Style.space(2)
              C.SettingsCaption {
                text: memory.modelData.text
                color: Commons.Color.foreground
                font.pixelSize: Style.font.body
              }
              C.SettingsCaption {
                visible: memory.normStatus === "pending" && memory.originalDiffers
                text: root.service ? root.service.tr("Original available", []) : ""
              }
              C.SettingsCaption {
                visible: memory.normStatus === "heuristic" && memory.modelData.text !== memory.sourceText
                text: root.service ? root.service.tr("cleaned", []) : ""
              }
              C.SettingsCaption {
                visible: memory.normStatus === "llm"
                text: root.service ? root.service.tr("Edited by AI", []) + (memory.sourceText !== "" ? root.service.tr(" · was: {}", [memory.sourceText]) : "") : ""
              }
            }
          }
          C.SettingsButtonsRow {
            id: memoryActions
            C.SettingsButton {
              visible: memory.normStatus !== "none" && memory.originalDiffers
              text: root.service ? root.service.tr("Restore original", []) : ""
              onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-restore-item", [memory.modelData.id])
            }
            C.SettingsButton {
              text: root.service ? root.service.tr("Edit", []) : ""
              onClicked: {
                if (!root.win || !root.service) return
                root.service.client.set(root.win.winId, "memory-edit-text", memory.modelData.text)
                root.service.client.set(root.win.winId, "memory-editing-id", memory.modelData.id)
              }
            }
            C.SettingsButton {
              text: root.service ? root.service.tr("Delete", []) : ""
              onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-delete-item", [memory.modelData.id])
            }
          }
        }
        C.SettingsFieldRow {
          visible: root.editingId === memory.modelData.id
          C.SettingsLineEdit {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            text: root.editText
            onTextEdited: if (root.win && root.service) root.service.client.set(root.win.winId, "memory-edit-text", text)
            onAccepted: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-edit-save", [memory.modelData.id, text])
          }
          C.SettingsButton {
            compact: true
            text: root.service ? root.service.tr("Save", []) : ""
            onClicked: if (root.win && root.service) root.service.client.call(root.win.winId, "memory-edit-save", [memory.modelData.id, root.editText])
          }
          C.SettingsButton {
            compact: true
            text: root.service ? root.service.tr("Cancel", []) : ""
            onClicked: if (root.win && root.service) root.service.client.set(root.win.winId, "memory-editing-id", -1)
          }
        }
      }
    }
  }
}
