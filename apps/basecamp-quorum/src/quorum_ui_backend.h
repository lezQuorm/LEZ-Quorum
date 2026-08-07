#pragma once

#include <QByteArray>
#include <QProcess>
#include <QString>
#include <QStringList>
#include <QTimer>

#include "logos_ui_plugin_context.h"
#include "rep_quorum_ui_source.h"

class QuorumUiBackend : public QuorumUiSimpleSource, public LogosUiPluginContext {
public:
    QuorumUiBackend();
    ~QuorumUiBackend() override;

    void onContextReady() override;

    bool configureQuorumBinary(QString path) override;
    bool configureWorkingDirectory(QString path) override;
    bool startConfigured(
        QString operation,
        QStringList arguments,
        QString binaryPath,
        QString workingDirectory
    ) override;
    bool start(QString operation, QStringList arguments) override;
    bool cancel() override;

private:
    bool ensureWorkingDirectory(const QString& path);
    void drainOutput();
    void failToStart(const QString& kind, const QString& message);
    void finish(int exitCode, QProcess::ExitStatus exitStatus);
    void stopProcess();

    QProcess* m_process;
    QTimer m_timeout;
    QByteArray m_stdout;
    QByteArray m_stderr;
    bool m_cancelRequested = false;
    bool m_timedOut = false;
};
