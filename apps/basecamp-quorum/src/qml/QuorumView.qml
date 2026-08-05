import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Logos.Controls
import Logos.Theme

Rectangle {
    id: root
    color: Theme.palette.background

    readonly property var backend: logos.module("quorum_ui")
    property bool ready: false

    function run(label, args) {
        if (!root.ready || !root.backend || root.backend.busy)
            return false
        return root.backend.start(label, args)
    }

    Connections {
        target: logos
        function onViewModuleReadyChanged(moduleName, isReady) {
            if (moduleName === "quorum_ui")
                root.ready = isReady && root.backend !== null
        }
    }

    Component.onCompleted: {
        root.ready = root.backend !== null
            && logos.isViewModuleReady("quorum_ui")
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacing.large
        spacing: Theme.spacing.medium

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing.medium

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                LogosText {
                    text: "Quorum Multisig"
                    color: Theme.palette.text
                    font.pixelSize: Theme.typography.titleText
                    font.weight: Theme.typography.weightBold
                }
                LogosText {
                    text: "Private M-of-N treasury for LEZ"
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                }
            }

            BusyIndicator {
                running: root.backend && root.backend.busy
                visible: running
                Layout.preferredWidth: 28
                Layout.preferredHeight: 28
            }

            LogosText {
                visible: root.backend && root.backend.busy
                text: root.backend ? root.backend.activeOperation : ""
                color: Theme.palette.textSecondary
                font.pixelSize: Theme.typography.secondaryText
            }

            Button {
                text: "Cancel"
                visible: root.backend && root.backend.busy
                onClicked: root.backend.cancel()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing.small

            TextField {
                id: binaryField
                Layout.fillWidth: true
                text: "quorum"
                placeholderText: "quorum CLI binary path"
                selectByMouse: true
            }
            Button {
                text: "Use binary"
                enabled: root.ready && !(root.backend && root.backend.busy)
                onClicked: root.backend.configureQuorumBinary(binaryField.text)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing.small

            TextField {
                id: workingDirectoryField
                Layout.fillWidth: true
                text: root.backend ? root.backend.workingDirectory : ""
                placeholderText: "Private Quorum working directory"
                selectByMouse: true
            }
            Button {
                text: "Use directory"
                enabled: root.ready && !(root.backend && root.backend.busy)
                onClicked: root.backend.configureWorkingDirectory(workingDirectoryField.text)
            }
        }

        TabBar {
            id: tabs
            Layout.fillWidth: true
            enabled: root.ready

            TabButton { text: "Create" }
            TabButton { text: "Propose" }
            TabButton { text: "Approve" }
            TabButton { text: "Rotate" }
            TabButton { text: "State" }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabs.currentIndex

            Item {
                // --- Create ---
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacing.medium
                    spacing: Theme.spacing.medium

                    LogosText {
                        text: "Create a private M-of-N multisig"
                        color: Theme.palette.text
                        font.pixelSize: Theme.typography.secondaryText
                    }

                    GridLayout {
                        columns: 2
                        columnSpacing: Theme.spacing.medium
                        rowSpacing: Theme.spacing.small

                        LogosText { text: "Threshold (M)"; color: Theme.palette.textSecondary }
                        SpinBox { id: thresholdSpinner; from: 1; to: 9; value: 2 }

                        LogosText { text: "Members (N)"; color: Theme.palette.textSecondary }
                        SpinBox { id: memberSpinner; from: 1; to: 9; value: 3 }
                    }

                    Button {
                        text: "Create multisig"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run(
                            "create",
                            ["create",
                             "--threshold", String(thresholdSpinner.value),
                             "--members", String(memberSpinner.value),
                             "--tiers", '[{"id":1,"threshold":2,"max_amount":1000}]'])
                    }
                }
            }

            Item {
                // --- Propose ---
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacing.medium
                    spacing: Theme.spacing.medium

                    LogosText {
                        text: "Open a treasury proposal"
                        color: Theme.palette.text
                        font.pixelSize: Theme.typography.secondaryText
                    }

                    GridLayout {
                        columns: 2
                        columnSpacing: Theme.spacing.medium
                        rowSpacing: Theme.spacing.small

                        LogosText { text: "Recipient"; color: Theme.palette.textSecondary }
                        TextField { id: recipientField; Layout.fillWidth: true; placeholderText: "64-hex recipient" }

                        LogosText { text: "Amount"; color: Theme.palette.textSecondary }
                        SpinBox { id: amountSpinner; from: 1; to: 1_000_000; value: 100 }

                        LogosText { text: "Tier"; color: Theme.palette.textSecondary }
                        SpinBox { id: tierSpinner; from: 1; to: 9; value: 1 }
                    }

                    Button {
                        text: "Propose transfer"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run(
                            "propose",
                            ["propose",
                             "--action", "transfer",
                             "--recipient", recipientField.text,
                             "--amount", String(amountSpinner.value),
                             "--tier", String(tierSpinner.value)])
                    }
                }
            }

            Item {
                // --- Approve ---
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacing.medium
                    spacing: Theme.spacing.medium

                    LogosText {
                        text: "Approve as a shielded member (client-side ZK proof)"
                        color: Theme.palette.text
                        font.pixelSize: Theme.typography.secondaryText
                    }

                    GridLayout {
                        columns: 2
                        columnSpacing: Theme.spacing.medium
                        rowSpacing: Theme.spacing.small

                        LogosText { text: "Member index"; color: Theme.palette.textSecondary }
                        SpinBox { id: memberIdx; from: 0; to: 9; value: 0 }

                        LogosText { text: "Proposal id"; color: Theme.palette.textSecondary }
                        SpinBox { id: proposalIdx; from: 0; to: 999; value: 0 }
                    }

                    Button {
                        text: "Approve"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run(
                            "approve",
                            ["approve",
                             "--member", String(memberIdx.value),
                             "--proposal", String(proposalIdx.value)])
                    }

                    Button {
                        text: "Execute proposal"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run(
                            "execute",
                            ["execute", "--proposal", String(proposalIdx.value)])
                    }
                }
            }

            Item {
                // --- Rotate ---
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacing.medium
                    spacing: Theme.spacing.medium

                    LogosText {
                        text: "Rotate members privately (new commitment root only)"
                        color: Theme.palette.text
                        font.pixelSize: Theme.typography.secondaryText
                    }

                    Button {
                        text: "New member root"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run("new-root", ["new-root", "--members", String(memberSpinner.value)])
                    }

                    GridLayout {
                        columns: 2
                        columnSpacing: Theme.spacing.medium
                        rowSpacing: Theme.spacing.small

                        LogosText { text: "New member root"; color: Theme.palette.textSecondary }
                        TextField {
                            id: newRootField
                            Layout.fillWidth: true
                            placeholderText: "64-hex new member root"
                            selectByMouse: true
                        }
                    }

                    Button {
                        text: "Propose rotation"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run(
                            "rotate",
                            ["propose",
                             "--action", "rotate",
                             "--new-member-root", newRootField.text,
                             "--new-member-count", String(memberSpinner.value)])
                    }

                    Button {
                        text: "Activate replacement keys"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run("activate-rotation", ["activate-rotation"])
                    }
                }
            }

            Item {
                // --- State ---
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacing.medium
                    spacing: Theme.spacing.medium

                    LogosText {
                        text: "Multisig state"
                        color: Theme.palette.text
                        font.pixelSize: Theme.typography.secondaryText
                    }

                    Button {
                        text: "Show state"
                        enabled: root.ready && !(root.backend && root.backend.busy)
                        onClicked: root.run("info", ["info"])
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 140
            color: Theme.palette.backgroundSecondary
            border.color: Theme.palette.borderSecondary
            border.width: 1
            radius: 4

            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spacing.small

                TextArea {
                    readOnly: true
                    selectByMouse: true
                    wrapMode: TextEdit.WrapAnywhere
                    color: Theme.palette.text
                    text: {
                        if (!root.backend)
                            return "Unavailable"
                        var output = root.backend.lastOutput || ""
                        var error = root.backend.lastError || ""
                        if (output.length > 0 && error.length > 0)
                            return output + "\n" + error
                        if (error.length > 0)
                            return error
                        return output.length > 0 ? output : "Ready"
                    }
                    background: null
                }
            }
        }
    }
}
