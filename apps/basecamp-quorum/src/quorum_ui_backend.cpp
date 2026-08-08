#include "quorum_ui_backend.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcessEnvironment>
#include <QSettings>
#include <QStandardPaths>

#include <utility>

#ifdef Q_OS_UNIX
#include <csignal>
#include <unistd.h>
#endif

namespace {

constexpr char kSettingsOrg[] = "Logos";
constexpr char kSettingsApp[] = "Quorum";
constexpr char kBinaryKey[] = "quorumBinary";
constexpr char kWorkingDirectoryKey[] = "workingDirectory";
constexpr int kStartTimeoutMs = 10'000;
constexpr int kOperationTimeoutMs = 30 * 60 * 1'000;
constexpr int kCancelGraceMs = 3'000;

QString normalizedPath(const QString& value) {
    QString path = value.trimmed();
    if (path.isEmpty()) {
        return {};
    }
    if (path.startsWith(QLatin1Char('~'))) {
        path = QDir::homePath() + path.mid(1);
    }
    return QDir::cleanPath(path);
}

bool isAllowedCommand(const QString& command) {
    static const QStringList allowed = {
        QStringLiteral("create"),
        QStringLiteral("propose"),
        QStringLiteral("approve"),
        QStringLiteral("approve-all"),
        QStringLiteral("execute"),
        QStringLiteral("info"),
        QStringLiteral("new-root"),
        QStringLiteral("activate-rotation"),
        QStringLiteral("network"),
    };
    return allowed.contains(command);
}

bool isProofOperation(const QStringList& arguments) {
    if (arguments.isEmpty()) {
        return false;
    }
    if (arguments.first() == QStringLiteral("approve")
        || arguments.first() == QStringLiteral("approve-all")) {
        return true;
    }
    return arguments.first() == QStringLiteral("network")
        && (arguments.contains(QStringLiteral("approve"))
            || arguments.contains(QStringLiteral("approve-threshold")));
}

void signalProcessTree(QProcess* process, bool force) {
#ifdef Q_OS_UNIX
    const qint64 pid = process->processId();
    if (pid > 0
        && ::kill(-static_cast<pid_t>(pid), force ? SIGKILL : SIGTERM) == 0) {
        return;
    }
#endif
    if (force) {
        process->kill();
    } else {
        process->terminate();
    }
}

} // namespace

QuorumUiBackend::QuorumUiBackend()
    : QuorumUiSimpleSource(), m_process(new QProcess(this)) {
#ifdef Q_OS_UNIX
    m_process->setChildProcessModifier([]() { (void)::setpgid(0, 0); });
#endif

    QSettings settings(kSettingsOrg, kSettingsApp);
    QString binary = settings.value(kBinaryKey).toString();
    if (binary.isEmpty()) {
        binary = QStandardPaths::findExecutable(QStringLiteral("quorum"));
    }
    if (binary.isEmpty()) {
        binary = QStringLiteral("quorum");
    }
    QuorumUiSimpleSource::setQuorumBinary(binary);

    QString workDir = settings.value(kWorkingDirectoryKey).toString();
    if (workDir.isEmpty()) {
        workDir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
            + QStringLiteral("/quorum");
    }
    if (ensureWorkingDirectory(workDir)) {
        QuorumUiSimpleSource::setWorkingDirectory(workDir);
    }

    m_timeout.setSingleShot(true);
    connect(&m_timeout, &QTimer::timeout, this, [this]() {
        if (busy() && m_process->state() != QProcess::NotRunning) {
            m_timedOut = true;
            stopProcess();
        }
    });
    connect(m_process, &QProcess::readyReadStandardOutput, this, [this]() { drainOutput(); });
    connect(m_process, &QProcess::readyReadStandardError, this, [this]() { drainOutput(); });
    connect(
        m_process,
        qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
        this,
        [this](int code, QProcess::ExitStatus status) { finish(code, status); }
    );
    connect(m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) {
            failToStart(QStringLiteral("start"), m_process->errorString());
        }
    });
}

QuorumUiBackend::~QuorumUiBackend() {
    if (m_process->state() != QProcess::NotRunning) {
        signalProcessTree(m_process, false);
        if (!m_process->waitForFinished(1'000)) {
            signalProcessTree(m_process, true);
            m_process->waitForFinished(1'000);
        }
    }
}

void QuorumUiBackend::onContextReady() {
}

bool QuorumUiBackend::configureQuorumBinary(QString path) {
    if (busy()) {
        return false;
    }
    path = normalizedPath(path);
    const QString resolved = path.contains(QLatin1Char('/'))
        ? path
        : QStandardPaths::findExecutable(path);
    if (resolved.isEmpty() || !QFileInfo(resolved).isExecutable()) {
        QuorumUiSimpleSource::setFailureKind(QStringLiteral("configuration"));
        QuorumUiSimpleSource::setLastError(
            QStringLiteral("Quorum binary is not executable: %1").arg(path)
        );
        return false;
    }
    QuorumUiSimpleSource::setQuorumBinary(resolved);
    QuorumUiSimpleSource::setFailureKind({});
    QuorumUiSimpleSource::setLastError({});
    QSettings(kSettingsOrg, kSettingsApp).setValue(kBinaryKey, resolved);
    return true;
}

bool QuorumUiBackend::configureWorkingDirectory(QString path) {
    if (busy()) {
        return false;
    }
    path = normalizedPath(path);
    if (!ensureWorkingDirectory(path)) {
        QuorumUiSimpleSource::setFailureKind(QStringLiteral("configuration"));
        QuorumUiSimpleSource::setLastError(
            QStringLiteral("Cannot create a private working directory: %1").arg(path)
        );
        return false;
    }
    QuorumUiSimpleSource::setWorkingDirectory(path);
    QuorumUiSimpleSource::setFailureKind({});
    QuorumUiSimpleSource::setLastError({});
    QSettings(kSettingsOrg, kSettingsApp).setValue(kWorkingDirectoryKey, path);
    return true;
}

bool QuorumUiBackend::startConfigured(
    QString operation,
    QStringList arguments,
    QString binaryPath,
    QString workingDirectory
) {
    if (busy()) {
        return false;
    }
    if (!configureQuorumBinary(std::move(binaryPath))
        || !configureWorkingDirectory(std::move(workingDirectory))) {
        return false;
    }
    return start(std::move(operation), std::move(arguments));
}

bool QuorumUiBackend::start(QString operation, QStringList arguments) {
    if (busy()) {
        return false;
    }
    if (arguments.isEmpty() || !isAllowedCommand(arguments.first())) {
        QuorumUiSimpleSource::setFailureKind(QStringLiteral("validation"));
        QuorumUiSimpleSource::setLastError(QStringLiteral("Unsupported Quorum command"));
        return false;
    }
    const QString binary = quorumBinary();
    const QString resolved = binary.contains(QLatin1Char('/'))
        ? binary
        : QStandardPaths::findExecutable(binary);
    if (resolved.isEmpty() || !QFileInfo(resolved).isExecutable()) {
        QuorumUiSimpleSource::setFailureKind(QStringLiteral("configuration"));
        QuorumUiSimpleSource::setLastError(
            QStringLiteral("Quorum binary is not executable: %1").arg(binary)
        );
        return false;
    }
    if (!ensureWorkingDirectory(workingDirectory())) {
        QuorumUiSimpleSource::setFailureKind(QStringLiteral("configuration"));
        QuorumUiSimpleSource::setLastError(QStringLiteral("Quorum working directory is unavailable"));
        return false;
    }

    m_stdout.clear();
    m_stderr.clear();
    m_cancelRequested = false;
    m_timedOut = false;
    QuorumUiSimpleSource::setLastOutput({});
    QuorumUiSimpleSource::setLastError({});
    QuorumUiSimpleSource::setFailureKind({});
    QuorumUiSimpleSource::setLastExitCode(0);
    QuorumUiSimpleSource::setActiveOperation(operation.trimmed());
    QuorumUiSimpleSource::setBusy(true);

    QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    environment.insert(QStringLiteral("RISC0_DEV_MODE"), QStringLiteral("0"));
    m_process->setProcessEnvironment(environment);
    m_process->setWorkingDirectory(workingDirectory());
    m_process->setProgram(resolved);
    m_process->setArguments(arguments);
    m_process->setProcessChannelMode(QProcess::SeparateChannels);
    m_process->start();
    if (!m_process->waitForStarted(kStartTimeoutMs)) {
        failToStart(QStringLiteral("start"), m_process->errorString());
        return false;
    }
    if (isProofOperation(arguments)) {
        m_timeout.stop();
    } else {
        m_timeout.start(kOperationTimeoutMs);
    }
    return true;
}

bool QuorumUiBackend::cancel() {
    if (!busy() || m_process->state() == QProcess::NotRunning) {
        return false;
    }
    m_cancelRequested = true;
    stopProcess();
    return true;
}

bool QuorumUiBackend::ensureWorkingDirectory(const QString& path) {
    const QString clean = QDir::cleanPath(path);
    if (clean.isEmpty()
        || !QFileInfo(clean).isAbsolute()
        || clean == QDir::rootPath()
        || clean == QDir::homePath()
        || !QDir().mkpath(clean)) {
        return false;
    }
    return QFile::setPermissions(
        clean,
        QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner
    );
}

void QuorumUiBackend::drainOutput() {
    m_stdout.append(m_process->readAllStandardOutput());
    m_stderr.append(m_process->readAllStandardError());
    QuorumUiSimpleSource::setLastOutput(QString::fromUtf8(m_stdout));
    QuorumUiSimpleSource::setLastError(QString::fromUtf8(m_stderr));
}

void QuorumUiBackend::finish(int exitCode, QProcess::ExitStatus exitStatus) {
    if (!busy()) {
        return;
    }
    m_timeout.stop();
    drainOutput();
    const bool success = !m_cancelRequested && !m_timedOut
        && exitStatus == QProcess::NormalExit && exitCode == 0;
    const QString output = QString::fromUtf8(m_stdout).trimmed();
    QString error = QString::fromUtf8(m_stderr).trimmed();
    QString failure;
    if (m_timedOut) {
        failure = QStringLiteral("timeout");
        error = QStringLiteral("Quorum operation exceeded its time limit");
    } else if (m_cancelRequested) {
        failure = QStringLiteral("cancelled");
        error = QStringLiteral("Quorum operation was cancelled");
    } else if (!success) {
        failure = exitStatus == QProcess::CrashExit
            ? QStringLiteral("crash")
            : QStringLiteral("command");
        if (error.isEmpty()) {
            error = exitStatus == QProcess::CrashExit
                ? QStringLiteral("Quorum process crashed")
                : QStringLiteral("Quorum exited with code %1").arg(exitCode);
        }
    }
    QuorumUiSimpleSource::setLastOutput(output);
    QuorumUiSimpleSource::setLastError(error);
    QuorumUiSimpleSource::setFailureKind(failure);
    QuorumUiSimpleSource::setLastExitCode(exitCode);
    QuorumUiSimpleSource::setBusy(false);
    QuorumUiSimpleSource::setActiveOperation({});
    emit operationFinished(success, exitCode, output, error);
}

void QuorumUiBackend::failToStart(const QString& kind, const QString& message) {
    if (!busy()) {
        return;
    }
    m_timeout.stop();
    QuorumUiSimpleSource::setFailureKind(kind);
    QuorumUiSimpleSource::setLastError(message);
    QuorumUiSimpleSource::setLastExitCode(-1);
    QuorumUiSimpleSource::setBusy(false);
    QuorumUiSimpleSource::setActiveOperation({});
    emit operationFinished(false, -1, {}, message);
}

void QuorumUiBackend::stopProcess() {
    signalProcessTree(m_process, false);
    QTimer::singleShot(kCancelGraceMs, m_process, [process = m_process]() {
        if (process->state() != QProcess::NotRunning) {
            signalProcessTree(process, true);
        }
    });
}
