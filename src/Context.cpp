#include <filesystem>
#include <lbd/Context.hpp>

namespace lbd
{
  Context::Context(std::ostream &outputStream, const Options options)
    : options(options), diagnosticRenderer(outputStream, bufferManager),
      diagnosticEmitter(diagnosticRenderer) {}

  source::BufferManager &Context::getBufferManager() { return bufferManager; }

  diagnostics::DiagnosticEmitter &Context::getDiagnosticEmitter() { return diagnosticEmitter; }

  Options &Context::getOptions() { return options; }

  Logger &Context::getLogger() { return logger; }

  source::BufferId Context::loadFile(const std::string &path, const source::Range &range)
  {
    if (!std::filesystem::exists(source::getAbsolutePath(path)))
    {
      diagnosticEmitter.error(
        range,
        diagnostics::DiagnosticId::IO_PATH_DOES_NOT_EXISTS, path
      );
    }
    return bufferManager.loadFile(path);
  }
}