import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

// Surfshark VPN Omarchy bar widget with high-performance native Rust backend.
// Designed with a cyber-security aesthetic, distinctive telemetry overview,
// quick-action tiles, right-click context menu, and responsive layout.
Panel {
  id: root

  moduleName: "daailouivan.surfshark"
  ipcTarget: "daailouivan.surfshark"
  manageIpc: false

  readonly property string script: decodeURIComponent(Qt.resolvedUrl("bin/omarchy-surfshark").toString().replace(/^file:\/\//, ""))

  readonly property color foreground: bar ? bar.foreground : Color.foreground
  readonly property color dim: Qt.darker(foreground, 1.45)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family
  readonly property color hoverFill: Qt.rgba(foreground.r, foreground.g, foreground.b, 0.07)
  readonly property color activeFill: Qt.rgba(foreground.r, foreground.g, foreground.b, 0.14)
  readonly property color cardBackground: Qt.rgba(foreground.r, foreground.g, foreground.b, 0.04)
  readonly property color cardBorder: Qt.rgba(foreground.r, foreground.g, foreground.b, 0.10)
  readonly property color accentColor: Color.accent || Qt.rgba(0.05, 0.65, 0.95, 1.0)
  readonly property color successColor: "#10B981"
  readonly property color warningColor: "#F59E0B"

  readonly property color barIconColor: root.shown ? root.barForeground : Qt.darker(root.barForeground, 1.55)
  readonly property int refreshIntervalSec: Math.max(5, setting("refreshIntervalSec", 20))
  readonly property string usLocation: String(setting("usLocation", "us-nyc.prod.surfshark.com"))

  readonly property string usLabel: {
    for (var i = 0; i < root.allLocations.length; i++) {
      if (root.allLocations[i].connectionName === root.usLocation)
        return root.plain(root.allLocations[i].location)
    }
    var m = /^us-([a-z]+)\./.exec(root.usLocation)
    return m ? m[1].toUpperCase() : "United States"
  }

  // State from backend
  property bool configured: false
  property bool daemonReady: false
  property bool connected: false
  property string location: ""
  property string country: ""
  property string myCountry: ""
  property string myCountryCode: ""
  property string myCity: ""
  property string publicIp: ""
  property bool secured: false
  property var loadValue: null
  property var recentList: []
  property string lastError: ""

  // Location search
  property var allLocations: []
  property string query: ""
  readonly property int maxResults: 8
  readonly property var matches: {
    var q = root.query.trim().toLowerCase()
    if (q === "") return []
    var out = []
    for (var i = 0; i < root.allLocations.length; i++) {
      var l = root.allLocations[i]
      if (String(l.location).toLowerCase().indexOf(q) !== -1
          || String(l.country).toLowerCase().indexOf(q) !== -1
          || String(l.countryCode).toLowerCase().indexOf(q) === 0) {
        out.push(l)
      }
    }
    return out
  }
  readonly property var results: matches.slice(0, maxResults)
  readonly property int hiddenMatches: Math.max(0, matches.length - maxResults)

  property bool busy: false
  property int desired: -1
  readonly property bool shown: busy && desired >= 0 ? desired === 1 : connected

  readonly property string statusText: {
    if (!daemonReady) return "Daemon Offline"
    if (!configured) return "Setup Required"
    if (busy) return desired === 1 ? "Connecting..." : "Disconnecting..."
    if (connected) return location !== "" ? ("Protected · " + location) : "Protected"
    return "Disconnected"
  }

  function plain(s) {
    if (!s) return ""
    return String(s).replace(/[\u0000-\u001f\u007f-\u009f]/g, "").trim()
  }

  function refresh() {
    if (statusProcess.running) return
    statusProcess.command = [root.script, "status"]
    statusProcess.running = true
  }

  function runAction(verb) {
    if (actionProcess.running) return
    root.lastError = ""
    root.busy = true
    root.desired = verb === "off" ? 0 : 1
    actionProcess.command = verb === "usa" ? [root.script, verb, root.usLocation]
                                           : [root.script, verb]
    actionProcess.running = true
  }

  function connectTo(host) {
    if (actionProcess.running || !host) return
    root.lastError = ""
    root.busy = true
    root.desired = 1
    actionProcess.command = [root.script, "connect", host]
    actionProcess.running = true
  }

  function toggleVpn() {
    runAction(root.shown ? "off" : "on")
  }

  function openApp() {
    if (actionProcess.running) return
    actionProcess.command = [root.script, "open"]
    actionProcess.running = true
  }

  function loadLocations() {
    if (locationsProcess.running || root.allLocations.length > 0) return
    locationsProcess.command = [root.script, "locations"]
    locationsProcess.running = true
  }

  function clearRecent() {
    if (clearProcess.running) return
    clearProcess.command = [root.script, "recent-clear"]
    clearProcess.running = true
  }

  function applyStatus(text) {
    var d
    try {
      d = JSON.parse(text)
    } catch (e) {
      return
    }
    root.configured = d.configured === true
    root.daemonReady = d.daemon === true
    root.connected = d.connected === true
    root.location = root.plain(d.location)
    root.country = root.plain(d.country)
    root.myCountry = root.plain(d.myCountry)
    root.myCountryCode = root.plain(d.myCountryCode)
    root.myCity = root.plain(d.myCity)
    root.publicIp = root.plain(d.ip)
    root.secured = d.secured === true
    root.loadValue = d.load
    root.recentList = d.recent || []

    if (!actionProcess.running && root.desired >= 0) {
      if ((root.desired === 1) === root.connected) {
        root.desired = -1
        root.busy = false
      }
    }
  }

  Component.onCompleted: refresh()

  onOpenedChanged: {
    if (opened) {
      root.contextMenuOpen = false
      loadLocations()
    } else {
      query = ""
    }
  }

  Timer {
    interval: root.refreshIntervalSec * 1000
    running: true
    repeat: true
    onTriggered: root.refresh()
  }

  // Fail-safe timer to prevent busy state hang
  Timer {
    interval: 5000
    running: root.busy
    repeat: false
    onTriggered: {
      root.busy = false
      root.desired = -1
    }
  }

  Process {
    id: statusProcess
    running: false
    command: []
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: root.applyStatus(text)
    }
  }

  Process {
    id: actionProcess
    running: false
    command: []
    stderr: StdioCollector { id: actionErr; waitForEnd: true }
    onExited: function(exitCode) {
      root.busy = false
      if (exitCode !== 0) {
        root.desired = -1
        root.lastError = root.plain(String(actionErr.text || "").trim().split("\n").pop())
      }
      root.refresh()
    }
  }

  Process {
    id: locationsProcess
    running: false
    command: []
    stdout: StdioCollector {
      id: locationsOut
      waitForEnd: true
      onStreamFinished: {
        try {
          root.allLocations = JSON.parse(locationsOut.text)
        } catch (e) {
          root.allLocations = []
        }
      }
    }
  }

  Process {
    id: clearProcess
    running: false
    command: []
    onExited: root.refresh()
  }

  IpcHandler {
    target: root.ipcTarget
    function open(): void { root.open() }
    function close(): void { root.close() }
    function show(): void { root.open() }
    function hide(): void { root.close() }
    function toggle(): void { root.toggle() }
    function refresh(): string { root.refresh(); return "ok" }
    function on(): string { root.runAction("on"); return "ok" }
    function off(): string { root.runAction("off"); return "ok" }
    function fastest(): string { root.runAction("fastest"); return "ok" }
    function nearest(): string { root.runAction("nearest"); return "ok" }
    function same(): string { root.runAction("same"); return "ok" }
    function usa(): string { root.runAction("usa"); return "ok" }
    function random(): string { root.runAction("random"); return "ok" }
    function openApp(): string { root.openApp(); return "ok" }
  }

  // Right-click Quick Control Context Menu (Native Omarchy PopupCard for Wayland)
  property bool contextMenuOpen: false

  PopupCard {
    id: contextPopup
    anchorItem: barButton
    owner: root
    bar: root.bar
    open: root.contextMenuOpen
    padding: Style.space(8)
    contentWidth: contextPopup.fittedContentWidth(Style.space(240))
    contentHeight: contextPopup.fittedContentHeight(contextColumn.implicitHeight)

    Column {
      id: contextColumn
      width: parent.width
      spacing: Style.space(3)

      Item {
        width: parent.width
        implicitHeight: Style.space(24)

        RowLayout {
          anchors.fill: parent
          anchors.leftMargin: Style.space(6)
          anchors.rightMargin: Style.space(6)
          spacing: Style.space(6)

          SurfsharkIcon {
            Layout.preferredWidth: Style.space(14)
            Layout.preferredHeight: Style.space(14)
            color: root.barIconColor
            active: root.shown
            crossed: !root.shown
          }

          Text {
            text: "Surfshark Quick Menu"
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            font.bold: true
            Layout.fillWidth: true
          }

          Rectangle {
            width: Style.space(6)
            height: Style.space(6)
            radius: width / 2
            color: root.shown ? root.successColor : root.dim
          }
        }
      }

      Rectangle {
        width: parent.width
        height: 1
        color: root.cardBorder
        opacity: 0.5
      }

      ContextMenuItem {
        iconText: root.shown ? "⏹" : "⚡"
        text: root.shown ? "Disconnect VPN" : "Quick Connect"
        onTriggered: {
          root.contextMenuOpen = false
          root.toggleVpn()
        }
      }

      ContextMenuItem {
        iconText: "⚡"
        text: "Fastest Location"
        onTriggered: {
          root.contextMenuOpen = false
          root.runAction("fastest")
        }
      }

      ContextMenuItem {
        iconText: "🇺🇸"
        text: "United States (" + root.usLabel + ")"
        onTriggered: {
          root.contextMenuOpen = false
          root.runAction("usa")
        }
      }

      Rectangle {
        width: parent.width
        height: 1
        color: root.cardBorder
        opacity: 0.5
      }

      ContextMenuItem {
        iconText: "🖥️"
        text: "Open Surfshark App"
        onTriggered: {
          root.contextMenuOpen = false
          root.openApp()
        }
      }

      ContextMenuItem {
        iconText: "🔄"
        text: "Refresh Status"
        onTriggered: {
          root.contextMenuOpen = false
          root.refresh()
        }
      }

      ContextMenuItem {
        iconText: "⚙️"
        text: "Open Details Panel..."
        onTriggered: {
          root.contextMenuOpen = false
          root.open()
        }
      }
    }
  }

  implicitWidth: barButton.implicitWidth
  implicitHeight: barButton.implicitHeight

  BarIconButton {
    id: barButton
    anchors.fill: parent
    bar: root.bar
    tooltipText: "Surfshark — " + root.statusText
    iconComponent: Component {
      Item {
        SurfsharkIcon {
          anchors.centerIn: parent
          iconSize: Style.space(12)
          color: root.barIconColor
          active: root.shown
          crossed: !root.shown
        }
      }
    }
    onPressed: function(buttonCode) {
      if (buttonCode === Qt.RightButton) {
        root.contextMenuOpen = !root.contextMenuOpen
      } else if (buttonCode === Qt.MiddleButton) {
        root.refresh()
      } else {
        root.contextMenuOpen = false
        root.toggle()
      }
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: barButton
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(380))
    contentHeight: panel.fittedContentHeight(panelColumn.implicitHeight, Style.space(450))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onCloseRequested: root.close()
      onTabRequested: function(dir) { root.switchPanel(dir) }
      onTextKey: function(t) {
        var k = String(t).toLowerCase()
        if (k === "t") root.toggleVpn()
        else if (k === "f") root.runAction("fastest")
        else if (k === "n") root.runAction("nearest")
        else if (k === "s") root.runAction("same")
        else if (k === "u") root.runAction("usa")
        else if (k === "x") root.runAction("random")
        else if (k === "o") root.openApp()
        else if (k === "r") root.refresh()
        else if (k === "/") searchInput.forceActiveFocus()
      }

      Flickable {
        id: panelScroll
        anchors.fill: parent
        contentWidth: width
        contentHeight: panelColumn.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        interactive: contentHeight > height
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
          id: panelColumn
          width: panelScroll.width
          spacing: Style.space(10)
          padding: Style.space(12)

          // Unified Status & Telemetry Card
          Rectangle {
            width: parent.width - Style.space(24)
            anchors.horizontalCenter: parent.horizontalCenter
            implicitHeight: cardContent.implicitHeight + Style.space(18)
            radius: Style.space(10)
            color: root.cardBackground
            border.color: root.cardBorder
            border.width: 1

            ColumnLayout {
              id: cardContent
              anchors.fill: parent
              anchors.margins: Style.space(10)
              spacing: Style.space(8)

              // Top row: Shield, status header, switch
              RowLayout {
                Layout.fillWidth: true
                spacing: Style.space(10)

                SurfsharkIcon {
                  Layout.preferredWidth: Style.space(28)
                  Layout.preferredHeight: Style.space(28)
                  color: root.shown ? root.accentColor : root.dim
                  active: root.shown
                  crossed: !root.shown
                }

                ColumnLayout {
                  Layout.fillWidth: true
                  spacing: Style.space(2)

                  Text {
                    text: root.shown && root.location !== "" ? root.location : "Surfshark VPN"
                    color: root.foreground
                    font.family: root.fontFamily
                    font.pixelSize: Style.font.title
                    font.bold: true
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                  }

                  RowLayout {
                    spacing: Style.space(5)

                    Rectangle {
                      width: Style.space(6)
                      height: Style.space(6)
                      radius: width / 2
                      color: !root.daemonReady ? root.warningColor
                           : (root.shown ? root.successColor : root.dim)
                    }

                    Text {
                      text: root.statusText.toUpperCase()
                      color: root.dim
                      font.family: root.fontFamily
                      font.pixelSize: Style.font.caption
                      font.bold: true
                    }
                  }
                }

                ToggleSwitch {
                  checked: root.shown
                  busy: root.busy
                  enabled: root.configured && root.daemonReady
                  foreground: root.foreground
                  onToggled: root.toggleVpn()
                }
              }

              // Divider
              Rectangle {
                Layout.fillWidth: true
                height: 1
                color: root.cardBorder
                opacity: 0.5
              }

              // Telemetry Info Row
              RowLayout {
                Layout.fillWidth: true
                spacing: Style.space(12)

                RowLayout {
                  spacing: Style.space(4)
                  Text { text: "IP:"; color: root.dim; font.pixelSize: Style.font.caption; font.bold: true }
                  Text { text: root.publicIp || "—"; color: root.foreground; font.pixelSize: Style.font.caption; font.bold: true }
                }

                RowLayout {
                  Layout.fillWidth: true
                  spacing: Style.space(4)
                  Text { text: "LOC:"; color: root.dim; font.pixelSize: Style.font.caption; font.bold: true }
                  Text {
                    text: root.shown ? (root.location + (root.country ? (", " + root.country) : ""))
                                     : (root.myCity ? (root.myCity + ", " + root.myCountryCode) : (root.myCountry || "—"))
                    color: root.foreground
                    font.pixelSize: Style.font.caption
                    font.bold: true
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                  }
                }

                RowLayout {
                  visible: root.loadValue !== null
                  spacing: Style.space(4)
                  Text { text: "LOAD:"; color: root.dim; font.pixelSize: Style.font.caption; font.bold: true }
                  Text {
                    text: root.loadValue !== null ? (root.loadValue + "%") : "—"
                    color: root.accentColor
                    font.pixelSize: Style.font.caption
                    font.bold: true
                  }
                }
              }
            }
          }

          // Error banner
          Rectangle {
            width: parent.width - Style.space(24)
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.lastError !== ""
            implicitHeight: errorText.implicitHeight + Style.space(14)
            radius: Style.space(8)
            color: Qt.rgba(0.9, 0.2, 0.2, 0.12)
            border.color: Qt.rgba(0.9, 0.2, 0.2, 0.3)
            border.width: 1

            Text {
              id: errorText
              anchors.fill: parent
              anchors.margins: Style.space(8)
              text: root.lastError
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.bodySmall
              wrapMode: Text.WordWrap
            }
          }

          // Notice banner when daemon/config needs attention
          Rectangle {
            width: parent.width - Style.space(24)
            anchors.horizontalCenter: parent.horizontalCenter
            visible: !root.configured || !root.daemonReady
            implicitHeight: noticeText.implicitHeight + Style.space(14)
            radius: Style.space(8)
            color: Qt.rgba(root.warningColor.r, root.warningColor.g, root.warningColor.b, 0.10)
            border.color: Qt.rgba(root.warningColor.r, root.warningColor.g, root.warningColor.b, 0.25)
            border.width: 1

            Text {
              id: noticeText
              anchors.fill: parent
              anchors.margins: Style.space(8)
              text: !root.daemonReady
                ? "Surfshark daemon offline. Start official app or run: systemctl --user start surfsharkd"
                : "Open the official Surfshark app once to register account keys."
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.bodySmall
              wrapMode: Text.WordWrap
            }
          }

          // Quick Routing Actions (Full-width sleek tiles to prevent any truncation)
          Column {
            width: parent.width - Style.space(24)
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Style.space(6)
            visible: root.configured && root.daemonReady

            ActionRowTile {
              width: parent.width
              iconText: "⚡"
              title: "Fastest Location"
              subtitle: "Recommended route · Low latency"
              tag: "AUTO"
              onClicked: root.runAction("fastest")
            }

            ActionRowTile {
              width: parent.width
              iconText: "🇺🇸"
              title: "United States"
              subtitle: root.usLabel
              tag: "US"
              onClicked: root.runAction("usa")
            }

            ActionRowTile {
              width: parent.width
              iconText: "🖥️"
              title: "Open Desktop App"
              subtitle: "Full server list & account settings"
              tag: "APP"
              onClicked: root.openApp()
            }
          }

          // Recent Connections Section (Only shown when not actively filtering)
          Column {
            width: parent.width - Style.space(24)
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Style.space(6)
            visible: root.configured && root.daemonReady && root.recentList.length > 0 && root.query === ""

            RowLayout {
              width: parent.width
              Text {
                text: "RECENTS"
                color: root.dim
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
                font.bold: true
                Layout.fillWidth: true
              }

              Text {
                text: "CLEAR"
                color: root.dim
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
                font.bold: true
                MouseArea {
                  anchors.fill: parent
                  anchors.margins: -4
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.clearRecent()
                }
              }
            }

            Repeater {
              model: root.recentList.slice(0, 3)
              ActionRowTile {
                required property var modelData
                width: parent.width
                iconText: "🌐"
                title: root.plain(modelData.location || modelData.connectionName)
                subtitle: root.plain(modelData.country || "")
                tag: root.plain(modelData.countryCode || "")
                isCurrent: root.shown && root.plain(modelData.location) === root.location
                onClicked: root.connectTo(String(modelData.connectionName || modelData.location || ""))
              }
            }
          }

          // Search and All Locations Section
          Column {
            width: parent.width - Style.space(24)
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Style.space(6)
            visible: root.configured && root.daemonReady

            Rectangle {
              width: parent.width
              height: Style.space(34)
              radius: Style.space(6)
              color: root.cardBackground
              border.color: searchInput.activeFocus ? root.accentColor : root.cardBorder
              border.width: 1

              TextInput {
                id: searchInput
                anchors.fill: parent
                anchors.leftMargin: Style.space(10)
                anchors.rightMargin: Style.space(10)
                verticalAlignment: TextInput.AlignVCenter
                color: root.foreground
                font.family: root.fontFamily
                font.pixelSize: Style.font.bodySmall
                text: root.query
                onTextChanged: root.query = text

                Text {
                  visible: parent.text === ""
                  text: "Filter 100+ locations (/)..."
                  color: root.dim
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.bodySmall
                  anchors.verticalCenter: parent.verticalCenter
                }

                onAccepted: {
                  if (root.results.length > 0)
                    root.connectTo(String(root.results[0].connectionName || root.results[0].location || ""))
                }

                Keys.onPressed: function(event) {
                  if (event.key === Qt.Key_Escape) {
                    if (root.query !== "") root.query = ""
                    else keyCatcher.forceActiveFocus()
                    event.accepted = true
                  }
                }
              }
            }

            // Search Results (Only shown when query is present)
            Repeater {
              model: root.results
              ActionRowTile {
                required property var modelData
                width: parent.width
                iconText: "📍"
                title: root.plain(modelData.location)
                subtitle: root.plain(modelData.country)
                tag: (modelData.load !== undefined && modelData.load !== null)
                     ? (modelData.load + "%") : String(modelData.countryCode || "")
                isCurrent: root.shown && root.plain(modelData.location) === root.location
                onClicked: root.connectTo(String(modelData.connectionName || modelData.location || ""))
              }
            }

            Text {
              visible: root.hiddenMatches > 0
              text: "+" + root.hiddenMatches + " more matches"
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }
        }
      }
    }
  }

  // Modern, full-width action tile component with zero truncation
  component ActionRowTile: Rectangle {
    id: arow
    property string iconText: ""
    property string title: ""
    property string subtitle: ""
    property string tag: ""
    property bool isCurrent: false
    signal clicked()

    implicitHeight: arow.subtitle !== "" ? Style.space(38) : Style.space(34)
    radius: Style.space(6)
    color: amouse.containsMouse ? root.hoverFill : (arow.isCurrent ? root.activeFill : root.cardBackground)
    border.color: arow.isCurrent ? root.accentColor : root.cardBorder
    border.width: 1

    Behavior on color { ColorAnimation { duration: 100 } }

    RowLayout {
      anchors.fill: parent
      anchors.leftMargin: Style.space(10)
      anchors.rightMargin: Style.space(10)
      spacing: Style.space(10)

      Text {
        visible: arow.iconText !== ""
        text: arow.iconText
        font.pixelSize: Style.font.body
        Layout.alignment: Qt.AlignVCenter
      }

      ColumnLayout {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignVCenter
        spacing: 1

        Text {
          text: arow.title
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.bodySmall
          font.bold: arow.isCurrent || arow.subtitle !== ""
          elide: Text.ElideRight
          Layout.fillWidth: true
        }

        Text {
          visible: arow.subtitle !== ""
          text: arow.subtitle
          color: root.dim
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          elide: Text.ElideRight
          Layout.fillWidth: true
        }
      }

      Rectangle {
        visible: arow.tag !== ""
        implicitWidth: atagText.implicitWidth + Style.space(8)
        implicitHeight: Style.space(18)
        radius: Style.space(3)
        color: arow.isCurrent ? root.accentColor : Qt.rgba(root.foreground.r, root.foreground.g, root.foreground.b, 0.08)

        Text {
          id: atagText
          anchors.centerIn: parent
          text: arow.tag
          color: arow.isCurrent ? "#000000" : root.dim
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          font.bold: true
        }
      }
    }

    MouseArea {
      id: amouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onClicked: arow.clicked()
    }
  }

  // Themed context menu item delegate matching Omarchy style
  component ContextMenuItem: Rectangle {
    id: cmi
    property string iconText: ""
    property string text: ""
    signal triggered()

    width: parent.width
    implicitHeight: Style.space(28)
    radius: Math.max(2, Style.cornerRadius)
    color: cmimouse.containsMouse ? root.hoverFill : "transparent"

    Behavior on color { ColorAnimation { duration: 80 } }

    RowLayout {
      anchors.fill: parent
      anchors.leftMargin: Style.space(8)
      anchors.rightMargin: Style.space(8)
      spacing: Style.space(8)

      Text {
        text: cmi.iconText
        font.pixelSize: Style.font.bodySmall
        color: root.foreground
      }

      Text {
        text: cmi.text
        font.family: root.fontFamily
        font.pixelSize: Style.font.bodySmall
        color: root.foreground
        elide: Text.ElideRight
        Layout.fillWidth: true
      }
    }

    MouseArea {
      id: cmimouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onClicked: cmi.triggered()
    }
  }
}
