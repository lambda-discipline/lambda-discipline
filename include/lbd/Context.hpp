#pragma once

#include <lbd/Logger.hpp>
#include <lbd/Options.hpp>
#include <lbd/diagnostics/DiagnosticEmitter.hpp>
#include <lbd/source/BufferManager.hpp>

namespace lbd
{
  class Context
  {
  public:
    explicit Context(std::ostream &outputStream, Options options = {});

    [[nodiscard]] source::BufferManager &getBufferManager();

    [[nodiscard]] diagnostics::DiagnosticEmitter &getDiagnosticEmitter();

    [[nodiscard]] Options &getOptions();

    [[nodiscard]] Logger &getLogger();

    /// Emits error upon unable to load the file.
    [[nodiscard]] source::BufferId loadFile(const std::string &path, const source::Range &range);

  private:
    Options options;
    Logger logger;
    source::BufferManager bufferManager;
    diagnostics::DiagnosticRenderer diagnosticRenderer;
    diagnostics::DiagnosticEmitter diagnosticEmitter;
  };
}