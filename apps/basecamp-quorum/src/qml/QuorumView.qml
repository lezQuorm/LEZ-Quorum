import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import Logos.Controls
import Logos.Theme

Rectangle {
    id: root

    color: Theme.palette.background

    readonly property var backend: logos.module("quorum_ui")
    readonly property bool operationBusy: !!root.backend && root.backend.busy
    readonly property bool canRun: root.ready && !root.operationBusy
    readonly property bool hasError: !!root.backend
                                     && (root.backend.lastError || "").length > 0
                                     && !root.operationBusy
    readonly property color statusColor: root.operationBusy
                                          ? Theme.palette.primary
                                          : (root.hasError
                                             ? Theme.palette.error
                                             : (root.ready
                                                ? Theme.palette.success
                                                : Theme.palette.textMuted))
    readonly property bool testnetMode: modeTabs.currentIndex === 1
    readonly property string lastOutput: root.backend ? (root.backend.lastOutput || "") : ""
    readonly property bool liveThresholdMet: root.outputNumber("approvals")
                                                >= root.outputNumber("required_approvals")
                                             && root.outputNumber("required_approvals") > 0
                                             && root.outputValue("proposal_status") === "Active"
    property bool ready: false
    property bool testnetSessionAssigned: false
    property string lastRequestedOperation: ""
    property bool lastRequestSubmitted: false
    readonly property real workflowTabWidth: Math.max(
        96,
        (Math.min(Math.max(0, root.width - Theme.spacing.xxlarge), 1280)
         - (root.width >= 980 ? 364 : 0)
         - Theme.spacing.large * 2) / 5)

    function isHex64(value) {
        return /^[0-9a-fA-F]{64}$/.test((value || "").trim())
    }

    function operationName(value) {
        const names = {
            "create": "Creating multisig",
            "propose": "Opening proposal",
            "approve": "Generating approval proof",
            "execute": "Executing proposal",
            "new-root": "Generating member root",
            "rotate": "Opening rotation proposal",
            "activate-rotation": "Activating replacement keys",
            "info": "Refreshing treasury state",
            "network-health": "Checking LEZ testnet",
            "network-deployment": "Verifying gate deployment",
            "network-prepare": "Preparing testnet state",
            "network-deploy": "Deploying gate",
            "network-initialize": "Initializing constitution",
            "network-token": "Creating token",
            "network-recipient": "Initializing recipient",
            "network-vault": "Initializing vault",
            "network-fund": "Funding vault",
            "network-propose": "Opening testnet proposal",
            "network-approve": "Generating private approval",
            "network-execute": "Executing testnet proposal",
            "network-status": "Reading testnet state",
            "network-reconcile": "Reconciling transactions"
        }
        return names[value] || "Working"
    }

    function activityText() {
        if (!root.backend)
            return "Quorum backend unavailable"

        const output = root.backend.lastOutput || ""
        const error = root.backend.lastError || ""
        if (output.length > 0 && error.length > 0)
            return output + "\n\n" + error
        if (error.indexOf("state already exists") >= 0)
            return "This session is already prepared. Select New session, then prepare private state."
        if (error.length > 0)
            return error
        if (output.length > 0)
            return output
        if (root.operationBusy)
            return root.operationName(root.backend.activeOperation) + "..."
        return "No activity yet"
    }

    function syncRuntimeFields() {
        if (!root.backend)
            return
        if (!binaryField.textInput.activeFocus)
            binaryField.text = root.backend.quorumBinary || "quorum"
        if (!workingDirectoryField.textInput.activeFocus)
            workingDirectoryField.text = root.backend.workingDirectory || ""
        if (!testnetWorkingDirectoryField.textInput.activeFocus)
            testnetWorkingDirectoryField.text = root.backend.workingDirectory || ""
    }

    function applyRuntime() {
        if (!root.ready || !root.backend || root.operationBusy)
            return
        root.backend.configureQuorumBinary(binaryField.text)
        root.backend.configureWorkingDirectory(workingDirectoryField.text)
    }

    function sessionTimestamp() {
        const now = new Date()
        function pad(value, size) {
            let result = String(value)
            while (result.length < size)
                result = "0" + result
            return result
        }
        return String(now.getFullYear())
            + pad(now.getMonth() + 1, 2)
            + pad(now.getDate(), 2)
            + "-"
            + pad(now.getHours(), 2)
            + pad(now.getMinutes(), 2)
            + pad(now.getSeconds(), 2)
            + pad(now.getMilliseconds(), 3)
    }

    function newTestnetSession() {
        if (!root.ready || !root.backend || root.operationBusy)
            return false

        let current = (root.backend.workingDirectory || testnetWorkingDirectoryField.text || "").trim()
        const separator = current.lastIndexOf("/")
        const parentDirectory = separator > 0
                                ? current.slice(0, separator)
                                : "/tmp"
        const sessionDirectory = parentDirectory
                               + "/lez-quorum-testnet-"
                               + root.sessionTimestamp()

        testnetWorkingDirectoryField.text = sessionDirectory
        root.backend.configureWorkingDirectory(sessionDirectory)
        root.testnetSessionAssigned = true
        publicWriteCheck.checked = false
        setupAction.currentIndex = 0
        treasuryAction.currentIndex = 0
        tabs.currentIndex = 0
        return true
    }

    function prepareTestnetState() {
        if (!root.testnetSessionAssigned && !root.newTestnetSession())
            return false
        return root.runNetwork(
            "network-prepare",
            "prepare",
            ["--threshold", String(thresholdSpinner.value),
             "--members", String(memberSpinner.value),
             "--funding", "750",
             "--transfer", "250"],
            false)
    }

    function runSetupAction() {
        if (setupAction.currentIndex === 0) {
            return root.runNetwork(
                "network-deployment", "deployment", [], false)
        }
        return root.runNetwork(
            "network-initialize", "initialize", [], true)
    }

    function setupActionLabel() {
        if (setupAction.currentIndex === 0)
            return "Verify deployment"
        return publicWriteCheck.checked
             ? "Submit initialization"
             : "Preview initialization"
    }

    function treasuryCommand() {
        const commands = [
            "create-token",
            "initialize-recipient",
            "initialize-vault",
            "fund",
            "propose"
        ]
        return commands[treasuryAction.currentIndex]
    }

    function treasuryOperation() {
        const operations = [
            "network-token",
            "network-recipient",
            "network-vault",
            "network-fund",
            "network-propose"
        ]
        return operations[treasuryAction.currentIndex]
    }

    function treasuryActionLabel() {
        const labels = [
            "token",
            "recipient",
            "vault",
            "funding",
            "proposal"
        ]
        return (publicWriteCheck.checked ? "Submit " : "Preview ")
             + labels[treasuryAction.currentIndex]
    }

    function runTreasuryAction() {
        return root.runNetwork(
            root.treasuryOperation(),
            root.treasuryCommand(),
            [],
            true)
    }

    function run(label, args) {
        if (!root.canRun)
            return false
        root.lastRequestedOperation = label
        root.lastRequestSubmitted = args.indexOf("--confirm-public-write") >= 0
        root.backend.startConfigured(
            label,
            args,
            binaryField.text,
            root.testnetMode ? testnetWorkingDirectoryField.text : workingDirectoryField.text)
        return true
    }

    function networkArguments(command, args, publicWrite) {
        let values = ["network", "--target", "testnet"]
        const rpc = rpcField.text.trim()
        if (rpc.length > 0)
            values = values.concat(["--rpc", rpc])
        values.push(command)
        values = values.concat(args || [])
        if (publicWrite && publicWriteCheck.checked)
            values.push("--confirm-public-write")
        return values
    }

    function runNetwork(label, command, args, publicWrite) {
        return root.run(label, root.networkArguments(command, args, publicWrite))
    }

    function outputValue(key) {
        const lines = root.lastOutput.split("\n")
        const prefix = key + "="
        for (let index = lines.length - 1; index >= 0; --index) {
            if (lines[index].indexOf(prefix) === 0)
                return lines[index].slice(prefix.length).trim()
        }
        return ""
    }

    function outputNumber(key) {
        const value = Number(root.outputValue(key))
        return isNaN(value) ? 0 : value
    }

    component PrimaryButton: LogosButton {
        id: primaryButton

        implicitWidth: 156
        implicitHeight: 40
        radius: Theme.spacing.radiusMedium

        background: Rectangle {
            color: !primaryButton.enabled
                   ? Theme.palette.backgroundMuted
                   : (primaryButton.isActive
                      ? Theme.palette.primaryPressed
                      : Theme.palette.primary)
            radius: primaryButton.radius
        }

        contentItem: LogosText {
            text: primaryButton.text
            color: primaryButton.enabled
                   ? Theme.palette.backgroundBlack
                   : Theme.palette.textMuted
            font.pixelSize: Theme.typography.primaryText
            font.weight: Theme.typography.weightMedium
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    component FieldLabel: LogosText {
        color: Theme.palette.textSecondary
        font.pixelSize: Theme.typography.secondaryText
        font.weight: Theme.typography.weightMedium
    }

    Connections {
        target: logos

        function onViewModuleReadyChanged(moduleName, isReady) {
            if (moduleName !== "quorum_ui")
                return
            root.ready = isReady && root.backend !== null
            if (root.ready)
                root.syncRuntimeFields()
        }
    }

    Connections {
        target: root.backend
        ignoreUnknownSignals: true

        function onQuorumBinaryChanged() { root.syncRuntimeFields() }
        function onWorkingDirectoryChanged() { root.syncRuntimeFields() }
        function onOperationFinished(success, exitCode, output, error) {
            if (success && root.testnetMode) {
                if (root.lastRequestedOperation === "network-deployment") {
                    setupAction.currentIndex = 1
                } else if (root.lastRequestSubmitted
                           && root.lastRequestedOperation === "network-initialize") {
                    tabs.currentIndex = 1
                } else if (root.lastRequestSubmitted
                           && ["network-token", "network-recipient",
                               "network-vault", "network-fund",
                               "network-propose"].indexOf(root.lastRequestedOperation) >= 0) {
                    if (treasuryAction.currentIndex < 4)
                        treasuryAction.currentIndex += 1
                    else
                        tabs.currentIndex = 2
                } else if (root.lastRequestSubmitted
                           && root.lastRequestedOperation === "network-approve") {
                    if (memberIdx.value + 1 < thresholdSpinner.value)
                        memberIdx.value += 1
                    else
                        tabs.currentIndex = 3
                } else if (root.lastRequestSubmitted
                           && root.lastRequestedOperation === "network-execute") {
                    tabs.currentIndex = 4
                }
            }
            publicWriteCheck.checked = false
            root.lastRequestSubmitted = false
        }
    }

    onTestnetModeChanged: {
        publicWriteCheck.checked = false
        tabs.currentIndex = 0
        if (root.testnetMode && !root.testnetSessionAssigned)
            Qt.callLater(root.newTestnetSession)
    }

    Component.onCompleted: {
        root.ready = root.backend !== null
            && logos.isViewModuleReady("quorum_ui")
        root.syncRuntimeFields()
    }

    ColumnLayout {
        id: shell

        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.topMargin: Theme.spacing.xlarge
        anchors.bottomMargin: Theme.spacing.xlarge
        width: Math.max(0, Math.min(parent.width - Theme.spacing.xxlarge, 1280))
        spacing: Theme.spacing.large

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            spacing: Theme.spacing.medium

            Rectangle {
                Layout.preferredWidth: 36
                Layout.preferredHeight: 36
                color: Theme.palette.primary
                radius: Theme.spacing.radiusMedium

                LogosText {
                    anchors.centerIn: parent
                    text: "Q"
                    color: Theme.palette.backgroundBlack
                    font.pixelSize: Theme.typography.subtitleText
                    font.weight: Theme.typography.weightBold
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                LogosText {
                    text: "Quorum"
                    color: Theme.palette.text
                    font.pixelSize: Theme.typography.panelTitleText
                    font.weight: Theme.typography.weightBold
                }

                LogosText {
                    visible: root.width >= 560
                    text: "Private multisig treasury"
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                }
            }

            LogosTabBar {
                id: modeTabs

                Layout.preferredWidth: root.width >= 760 ? 230 : 190
                enabled: root.canRun

                LogosTabButton {
                    width: root.width >= 760 ? 115 : 95
                    text: "Local"
                }

                LogosTabButton {
                    width: root.width >= 760 ? 115 : 95
                    text: "LEZ Testnet"
                }
            }

            RowLayout {
                visible: root.width >= 760 || root.operationBusy
                spacing: Theme.spacing.small

                Rectangle {
                    Layout.preferredWidth: 8
                    Layout.preferredHeight: 8
                    color: root.statusColor
                    radius: 4
                }

                LogosText {
                    text: root.operationBusy
                          ? root.operationName(root.backend.activeOperation)
                          : (root.hasError ? "Action required" : (root.ready ? "Ready" : "Connecting"))
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                }

                LogosButton {
                    visible: root.operationBusy
                    text: "Cancel"
                    radius: Theme.spacing.radiusMedium
                    Layout.preferredWidth: 76
                    Layout.preferredHeight: 34
                    onClicked: root.backend.cancel()
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: runtimeContent.implicitHeight + Theme.spacing.large * 2
            color: Theme.palette.backgroundSecondary
            border.color: root.hasError && root.backend.failureKind === "configuration"
                          ? Theme.palette.error
                          : Theme.palette.borderSecondary
            border.width: 1
            radius: Theme.spacing.radiusMedium

            ColumnLayout {
                id: runtimeContent

                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: Theme.spacing.large
                anchors.rightMargin: Theme.spacing.large
                spacing: Theme.spacing.medium

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing.small

                    LogosText {
                        Layout.fillWidth: true
                        text: root.testnetMode ? "Testnet session" : "Runtime"
                        color: Theme.palette.text
                        font.pixelSize: Theme.typography.primaryText
                        font.weight: Theme.typography.weightMedium
                    }

                    LogosButton {
                        visible: root.testnetMode
                        text: "Check RPC"
                        radius: Theme.spacing.radiusMedium
                        enabled: root.canRun
                        Layout.preferredWidth: 104
                        Layout.preferredHeight: 34
                        onClicked: root.runNetwork(
                            "network-health", "health", [], false)
                    }

                    LogosButton {
                        visible: root.testnetMode
                        text: "New session"
                        radius: Theme.spacing.radiusMedium
                        enabled: root.canRun
                        Layout.preferredWidth: 112
                        Layout.preferredHeight: 34
                        onClicked: root.newTestnetSession()
                    }

                    LogosButton {
                        visible: !root.testnetMode
                        text: "Apply"
                        radius: Theme.spacing.radiusMedium
                        enabled: root.canRun
                        Layout.preferredWidth: 76
                        Layout.preferredHeight: 34
                        onClicked: root.applyRuntime()
                    }
                }

                GridLayout {
                    visible: !root.testnetMode
                    Layout.fillWidth: true
                    columns: width >= 720 ? 2 : 1
                    columnSpacing: Theme.spacing.medium
                    rowSpacing: Theme.spacing.small

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: Theme.spacing.tiny

                        FieldLabel { text: "CLI binary" }

                        LogosTextField {
                            id: binaryField
                            Layout.fillWidth: true
                            placeholderText: "/absolute/path/to/quorum"
                            textInput.selectByMouse: true
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: Theme.spacing.tiny

                        FieldLabel { text: "Private state directory" }

                        LogosTextField {
                            id: workingDirectoryField
                            Layout.fillWidth: true
                            placeholderText: "/absolute/path/to/state"
                            textInput.selectByMouse: true
                        }
                    }
                }

                GridLayout {
                    visible: root.testnetMode
                    Layout.fillWidth: true
                    columns: width >= 720 ? 2 : 1
                    columnSpacing: Theme.spacing.medium
                    rowSpacing: Theme.spacing.small

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: Theme.spacing.tiny

                        FieldLabel { text: "Private session directory" }

                        LogosTextField {
                            id: testnetWorkingDirectoryField
                            Layout.fillWidth: true
                            placeholderText: "/absolute/path/to/session"
                            textInput.selectByMouse: true
                            onTextChanged: {
                                if (activeFocus)
                                    root.testnetSessionAssigned = true
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: Theme.spacing.tiny

                        FieldLabel { text: "Sequencer RPC" }

                        LogosTextField {
                            id: rpcField
                            Layout.fillWidth: true
                            text: "https://testnet.lez.logos.co"
                            textInput.selectByMouse: true
                        }
                    }
                }

                LogosSwitch {
                    id: publicWriteCheck

                    visible: root.testnetMode
                    enabled: root.canRun
                    text: "Submit next action to LEZ testnet"
                }
            }
        }

        GridLayout {
            id: workspace

            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: width >= 980 ? 2 : 1
            columnSpacing: Theme.spacing.large
            rowSpacing: Theme.spacing.large

            Rectangle {
                Layout.row: 0
                Layout.column: 0
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 0
                Layout.minimumHeight: 440
                color: Theme.palette.backgroundSecondary
                border.color: Theme.palette.borderSecondary
                border.width: 1
                radius: Theme.spacing.radiusMedium

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0

                    LogosTabBar {
                        id: tabs

                        Layout.fillWidth: true
                        Layout.leftMargin: Theme.spacing.large
                        Layout.rightMargin: Theme.spacing.large
                        Layout.topMargin: Theme.spacing.small
                        enabled: root.ready

                        LogosTabButton {
                            width: root.workflowTabWidth
                            text: root.testnetMode ? "1  Setup" : "Create"
                        }
                        LogosTabButton {
                            width: root.workflowTabWidth
                            text: root.testnetMode ? "2  Treasury" : "Propose"
                        }
                        LogosTabButton {
                            width: root.workflowTabWidth
                            text: root.testnetMode ? "3  Approve" : "Approve"
                        }
                        LogosTabButton {
                            width: root.workflowTabWidth
                            text: root.testnetMode ? "4  Execute" : "Rotate"
                        }
                        LogosTabButton {
                            width: root.workflowTabWidth
                            text: root.testnetMode ? "5  State" : "State"
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.palette.borderSecondary
                    }

                    StackLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        currentIndex: tabs.currentIndex

                        Item {
                            ColumnLayout {
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: Theme.spacing.xlarge
                                spacing: root.testnetMode
                                         ? Theme.spacing.medium
                                         : Theme.spacing.xlarge

                                ColumnLayout {
                                    spacing: Theme.spacing.tiny

                                    LogosText {
                                        text: root.testnetMode ? "Testnet setup" : "Create multisig"
                                        color: Theme.palette.text
                                        font.pixelSize: Theme.typography.subtitleText
                                        font.weight: Theme.typography.weightMedium
                                    }

                                    LogosText {
                                        text: root.testnetMode
                                              ? "LEZ v0.2.2 private treasury"
                                              : "Define the approval policy and member set."
                                        color: Theme.palette.textSecondary
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacing.medium

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacing.tiny
                                        FieldLabel { text: "Threshold" }
                                        LogosSpinBox {
                                            id: thresholdSpinner
                                            Layout.fillWidth: true
                                            from: 1
                                            to: memberSpinner.value
                                            value: 2
                                        }
                                    }

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacing.tiny
                                        FieldLabel { text: "Members" }
                                        LogosSpinBox {
                                            id: memberSpinner
                                            Layout.fillWidth: true
                                            from: 1
                                            to: 9
                                            value: 3
                                        }
                                    }
                                }

                                LogosText {
                                    text: thresholdSpinner.value + " of " + memberSpinner.value
                                          + " approvals required"
                                    color: Theme.palette.textSecondary
                                    font.pixelSize: Theme.typography.secondaryText
                                }

                                PrimaryButton {
                                    Layout.preferredWidth: root.testnetMode ? 200 : implicitWidth
                                    text: root.testnetMode
                                          ? "Prepare private state"
                                          : "Create multisig"
                                    enabled: root.canRun
                                    onClicked: {
                                        if (root.testnetMode) {
                                            root.prepareTestnetState()
                                        } else {
                                            root.run(
                                                "create",
                                                ["create",
                                                 "--threshold", String(thresholdSpinner.value),
                                                 "--members", String(memberSpinner.value),
                                                 "--tiers", "[{\"id\":1,\"threshold\":" + String(thresholdSpinner.value) + ",\"max_amount\":1000}]"])
                                        }
                                    }
                                }

                                GridLayout {
                                    visible: root.testnetMode
                                    Layout.fillWidth: true
                                    columns: width >= 520 ? 2 : 1
                                    columnSpacing: Theme.spacing.small
                                    rowSpacing: Theme.spacing.small

                                    LogosComboBox {
                                        id: setupAction
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: 40
                                        model: [
                                            "Verify deployed gate",
                                            "Initialize treasury"
                                        ]
                                        currentIndex: 0
                                    }

                                    PrimaryButton {
                                        Layout.fillWidth: true
                                        text: root.setupActionLabel()
                                        enabled: root.canRun
                                        onClicked: root.runSetupAction()
                                    }
                                }
                            }
                        }

                        Item {
                            ColumnLayout {
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: Theme.spacing.xlarge
                                spacing: root.testnetMode
                                         ? Theme.spacing.medium
                                         : Theme.spacing.xlarge

                                ColumnLayout {
                                    spacing: Theme.spacing.tiny

                                    LogosText {
                                        text: root.testnetMode ? "Treasury lifecycle" : "Propose transfer"
                                        color: Theme.palette.text
                                        font.pixelSize: Theme.typography.subtitleText
                                        font.weight: Theme.typography.weightMedium
                                    }

                                    LogosText {
                                        text: root.testnetMode
                                              ? "Token, vault, funding, and proposal"
                                              : "Open a treasury payment for member approval."
                                        color: Theme.palette.textSecondary
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                ColumnLayout {
                                    visible: !root.testnetMode
                                    Layout.fillWidth: true
                                    spacing: Theme.spacing.tiny

                                    FieldLabel { text: "Recipient" }

                                    LogosTextField {
                                        id: recipientField
                                        Layout.fillWidth: true
                                        placeholderText: "64-character hex address"
                                        textInput.selectByMouse: true
                                    }

                                    LogosText {
                                        visible: recipientField.text.length > 0
                                                 && !root.isHex64(recipientField.text)
                                        text: "Recipient must be exactly 64 hexadecimal characters."
                                        color: Theme.palette.error
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                RowLayout {
                                    visible: !root.testnetMode
                                    Layout.fillWidth: true
                                    spacing: Theme.spacing.medium

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacing.tiny
                                        FieldLabel { text: "Amount" }
                                        LogosSpinBox {
                                            id: amountSpinner
                                            Layout.fillWidth: true
                                            from: 1
                                            to: 1_000_000
                                            value: root.testnetMode ? 250 : 500
                                            enabled: !root.testnetMode
                                        }
                                    }

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacing.tiny
                                        FieldLabel { text: "Policy tier" }
                                        LogosSpinBox {
                                            id: tierSpinner
                                            Layout.fillWidth: true
                                            from: 1
                                            to: 9
                                            value: 1
                                        }
                                    }
                                }

                                PrimaryButton {
                                    visible: !root.testnetMode
                                    text: "Propose transfer"
                                    enabled: root.canRun && root.isHex64(recipientField.text)
                                    onClicked: root.run(
                                        "propose",
                                        ["propose",
                                         "--action", "transfer",
                                         "--recipient", recipientField.text.trim(),
                                         "--amount", String(amountSpinner.value),
                                         "--tier", String(tierSpinner.value)])
                                }

                                LogosText {
                                    visible: root.testnetMode
                                    text: "750 funded  /  250 transfer  /  tier 1"
                                    color: Theme.palette.textSecondary
                                    font.pixelSize: Theme.typography.secondaryText
                                }

                                GridLayout {
                                    visible: root.testnetMode
                                    Layout.fillWidth: true
                                    columns: width >= 520 ? 2 : 1
                                    columnSpacing: Theme.spacing.small
                                    rowSpacing: Theme.spacing.small

                                    LogosComboBox {
                                        id: treasuryAction
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: 40
                                        model: [
                                            "Create token",
                                            "Initialize recipient",
                                            "Initialize vault",
                                            "Fund vault",
                                            "Open proposal"
                                        ]
                                        currentIndex: 0
                                    }

                                    PrimaryButton {
                                        Layout.fillWidth: true
                                        text: root.treasuryActionLabel()
                                        enabled: root.canRun
                                        onClicked: root.runTreasuryAction()
                                    }
                                }
                            }
                        }

                        Item {
                            ColumnLayout {
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: Theme.spacing.xlarge
                                spacing: Theme.spacing.xlarge

                                ColumnLayout {
                                    spacing: Theme.spacing.tiny

                                    LogosText {
                                        text: root.testnetMode ? "Private approval" : "Approve proposal"
                                        color: Theme.palette.text
                                        font.pixelSize: Theme.typography.subtitleText
                                        font.weight: Theme.typography.weightMedium
                                    }

                                    LogosText {
                                        text: root.testnetMode
                                              ? "Real RISC Zero proof bound to live proposal state"
                                              : "Create a private member approval proof or execute quorum."
                                        color: Theme.palette.textSecondary
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacing.medium

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacing.tiny
                                        FieldLabel { text: "Member index" }
                                        LogosSpinBox {
                                            id: memberIdx
                                            Layout.fillWidth: true
                                            from: 0
                                            to: 9
                                            value: 0
                                        }
                                    }

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacing.tiny
                                        FieldLabel { text: "Proposal ID" }
                                        LogosSpinBox {
                                            id: proposalIdx
                                            Layout.fillWidth: true
                                            from: 0
                                            to: 999
                                            value: 0
                                        }
                                    }
                                }

                                RowLayout {
                                    spacing: Theme.spacing.small

                                    PrimaryButton {
                                        text: root.testnetMode
                                              ? (publicWriteCheck.checked
                                                 ? "Prove and submit"
                                                 : "Generate proof")
                                              : "Approve privately"
                                        enabled: root.canRun
                                        onClicked: {
                                            if (root.testnetMode) {
                                                root.runNetwork(
                                                    "network-approve",
                                                    "approve",
                                                    ["--member", String(memberIdx.value),
                                                     "--proposal", String(proposalIdx.value)],
                                                    true)
                                            } else {
                                                root.run(
                                                    "approve",
                                                    ["approve",
                                                     "--member", String(memberIdx.value),
                                                     "--proposal", String(proposalIdx.value)])
                                            }
                                        }
                                    }

                                    LogosButton {
                                        visible: !root.testnetMode
                                        text: "Execute"
                                        radius: Theme.spacing.radiusMedium
                                        enabled: root.canRun
                                        Layout.preferredWidth: 112
                                        Layout.preferredHeight: 40
                                        onClicked: root.run(
                                            "execute",
                                            ["execute", "--proposal", String(proposalIdx.value)])
                                    }

                                    LogosButton {
                                        visible: root.testnetMode
                                        text: "Refresh"
                                        radius: Theme.spacing.radiusMedium
                                        enabled: root.canRun
                                        Layout.preferredWidth: 100
                                        Layout.preferredHeight: 40
                                        onClicked: root.runNetwork(
                                            "network-status", "status", [], false)
                                    }
                                }

                                LogosText {
                                    visible: root.testnetMode
                                    text: root.outputNumber("approvals") + " / "
                                          + root.outputNumber("required_approvals")
                                          + " confirmed approvals"
                                    color: root.liveThresholdMet
                                           ? Theme.palette.success
                                           : Theme.palette.textSecondary
                                    font.pixelSize: Theme.typography.secondaryText
                                }
                            }
                        }

                        Item {
                            ColumnLayout {
                                visible: !root.testnetMode
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: Theme.spacing.xlarge
                                spacing: Theme.spacing.xlarge

                                ColumnLayout {
                                    spacing: Theme.spacing.tiny

                                    LogosText {
                                        text: "Rotate members"
                                        color: Theme.palette.text
                                        font.pixelSize: Theme.typography.subtitleText
                                        font.weight: Theme.typography.weightMedium
                                    }

                                    LogosText {
                                        text: "Replace the private member set without exposing identities."
                                        color: Theme.palette.textSecondary
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacing.tiny

                                    FieldLabel { text: "New member root" }

                                    LogosTextField {
                                        id: newRootField
                                        Layout.fillWidth: true
                                        placeholderText: "64-character commitment root"
                                        textInput.selectByMouse: true
                                    }

                                    LogosText {
                                        visible: newRootField.text.length > 0
                                                 && !root.isHex64(newRootField.text)
                                        text: "Member root must be exactly 64 hexadecimal characters."
                                        color: Theme.palette.error
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                RowLayout {
                                    spacing: Theme.spacing.small

                                    LogosButton {
                                        text: "Generate root"
                                        radius: Theme.spacing.radiusMedium
                                        enabled: root.canRun
                                        Layout.preferredWidth: 132
                                        Layout.preferredHeight: 40
                                        onClicked: root.run(
                                            "new-root",
                                            ["new-root", "--members", String(memberSpinner.value)])
                                    }

                                    PrimaryButton {
                                        text: "Propose rotation"
                                        enabled: root.canRun && root.isHex64(newRootField.text)
                                        onClicked: root.run(
                                            "rotate",
                                            ["propose",
                                             "--action", "rotate",
                                             "--new-member-root", newRootField.text.trim(),
                                             "--new-member-count", String(memberSpinner.value)])
                                    }
                                }

                                LogosButton {
                                    text: "Activate replacement keys"
                                    radius: Theme.spacing.radiusMedium
                                    enabled: root.canRun
                                    Layout.preferredWidth: 210
                                    Layout.preferredHeight: 40
                                    onClicked: root.run(
                                        "activate-rotation",
                                        ["activate-rotation"])
                                }
                            }

                            ColumnLayout {
                                visible: root.testnetMode
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: Theme.spacing.xlarge
                                spacing: Theme.spacing.xlarge

                                ColumnLayout {
                                    spacing: Theme.spacing.tiny

                                    LogosText {
                                        text: "Execute proposal"
                                        color: Theme.palette.text
                                        font.pixelSize: Theme.typography.subtitleText
                                        font.weight: Theme.typography.weightMedium
                                    }

                                    LogosText {
                                        text: root.outputNumber("approvals") + " of "
                                              + root.outputNumber("required_approvals")
                                              + " approvals confirmed"
                                        color: root.liveThresholdMet
                                               ? Theme.palette.success
                                               : Theme.palette.textSecondary
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                RowLayout {
                                    spacing: Theme.spacing.small

                                    LogosButton {
                                        text: "Refresh"
                                        radius: Theme.spacing.radiusMedium
                                        enabled: root.canRun
                                        Layout.preferredWidth: 100
                                        Layout.preferredHeight: 40
                                        onClicked: root.runNetwork(
                                            "network-status", "status", [], false)
                                    }

                                    PrimaryButton {
                                        text: publicWriteCheck.checked
                                              ? "Submit execution"
                                              : "Prepare execution"
                                        enabled: root.canRun && root.liveThresholdMet
                                        onClicked: root.runNetwork(
                                            "network-execute",
                                            "execute",
                                            ["--proposal", String(proposalIdx.value)],
                                            true)
                                    }
                                }

                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 1
                                    color: Theme.palette.borderSecondary
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacing.tiny

                                    FieldLabel { text: "Transaction label" }

                                    LogosTextField {
                                        id: reconcileLabel
                                        Layout.fillWidth: true
                                        placeholderText: "Optional for status query"
                                        textInput.selectByMouse: true
                                    }
                                }

                                RowLayout {
                                    spacing: Theme.spacing.small

                                    LogosButton {
                                        text: "Reconcile"
                                        radius: Theme.spacing.radiusMedium
                                        enabled: root.canRun
                                        Layout.preferredWidth: 112
                                        Layout.preferredHeight: 40
                                        onClicked: root.runNetwork(
                                            "network-reconcile",
                                            "reconcile",
                                            reconcileLabel.text.trim().length > 0
                                                ? ["--label", reconcileLabel.text.trim()]
                                                : [],
                                            false)
                                    }

                                    LogosButton {
                                        text: "Resubmit exact"
                                        radius: Theme.spacing.radiusMedium
                                        enabled: root.canRun
                                                 && publicWriteCheck.checked
                                                 && reconcileLabel.text.trim().length > 0
                                        Layout.preferredWidth: 132
                                        Layout.preferredHeight: 40
                                        onClicked: root.runNetwork(
                                            "network-reconcile",
                                            "reconcile",
                                            ["--label", reconcileLabel.text.trim(),
                                             "--resubmit-unconfirmed"],
                                            true)
                                    }
                                }
                            }
                        }

                        Item {
                            ColumnLayout {
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: Theme.spacing.xlarge
                                spacing: Theme.spacing.xlarge

                                ColumnLayout {
                                    spacing: Theme.spacing.tiny

                                    LogosText {
                                        text: root.testnetMode ? "Live testnet state" : "Treasury state"
                                        color: Theme.palette.text
                                        font.pixelSize: Theme.typography.subtitleText
                                        font.weight: Theme.typography.weightMedium
                                    }

                                    LogosText {
                                        text: root.testnetMode
                                              ? "Block, accounts, balances, approvals, and transaction journal"
                                              : "Inspect balances, members, policy tiers, and proposals."
                                        color: Theme.palette.textSecondary
                                        font.pixelSize: Theme.typography.secondaryText
                                    }
                                }

                                PrimaryButton {
                                    text: "Refresh state"
                                    enabled: root.canRun
                                    onClicked: {
                                        if (root.testnetMode) {
                                            root.runNetwork(
                                                "network-status", "status", [], false)
                                        } else {
                                            root.run("info", ["info"])
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.row: workspace.columns === 2 ? 0 : 1
                Layout.column: workspace.columns === 2 ? 1 : 0
                Layout.fillWidth: workspace.columns === 1
                Layout.fillHeight: true
                Layout.minimumWidth: workspace.columns === 2 ? 320 : 0
                Layout.minimumHeight: workspace.columns === 2 ? 440 : 220
                Layout.preferredWidth: workspace.columns === 2 ? 340 : workspace.width
                color: Theme.palette.backgroundInset
                border.color: Theme.palette.borderSecondary
                border.width: 1
                radius: Theme.spacing.radiusMedium

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacing.large
                    spacing: Theme.spacing.medium

                    RowLayout {
                        Layout.fillWidth: true

                        LogosText {
                            Layout.fillWidth: true
                            text: "Activity"
                            color: Theme.palette.text
                            font.pixelSize: Theme.typography.primaryText
                            font.weight: Theme.typography.weightMedium
                        }

                        LogosButton {
                            visible: root.activityText().length > 0
                            text: "Copy"
                            radius: Theme.spacing.radiusMedium
                            Layout.preferredWidth: 64
                            Layout.preferredHeight: 30
                            onClicked: {
                                activityArea.selectAll()
                                activityArea.copy()
                                activityArea.deselect()
                            }
                        }

                        LogosText {
                            visible: !!root.backend && !root.operationBusy
                                     && ((root.backend.lastOutput || "").length > 0
                                         || (root.backend.lastError || "").length > 0)
                            text: root.hasError ? "Failed" : "Complete"
                            color: root.statusColor
                            font.pixelSize: Theme.typography.secondaryText
                            font.weight: Theme.typography.weightMedium
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: Theme.palette.borderSecondary
                    }

                    LogosScrollView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        TextArea {
                            id: activityArea
                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.WrapAnywhere
                            text: root.activityText()
                            color: root.hasError
                                   ? Theme.palette.error
                                   : Theme.palette.textSecondary
                            font.family: "monospace"
                            font.pixelSize: Theme.typography.secondaryText
                            background: null
                            leftPadding: 0
                            rightPadding: Theme.spacing.small
                            topPadding: 0
                            bottomPadding: 0
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spacing.small

                        Rectangle {
                            Layout.preferredWidth: 7
                            Layout.preferredHeight: 7
                            color: root.statusColor
                            radius: 4
                        }

                        LogosText {
                            Layout.fillWidth: true
                            text: root.operationBusy
                                  ? root.operationName(root.backend.activeOperation)
                                  : (root.hasError ? "Review the error above" : "Ready for next action")
                            color: Theme.palette.textTertiary
                            font.pixelSize: Theme.typography.secondaryText
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }
}
