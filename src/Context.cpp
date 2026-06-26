#include <filesystem>
#include <lbd/Context.hpp>
#include <lbd/Error.hpp>

namespace lbd
{
  Context::Context(std::ostream &outputStream, const Options options)
    : options(options), diagnosticRenderer(outputStream, bufferManager),
      diagnosticEmitter(diagnosticRenderer, true) {}

  source::BufferManager &Context::getBufferManager() { return bufferManager; }

  diagnostics::DiagnosticEmitter &Context::getDiagnosticEmitter() { return diagnosticEmitter; }

  Options &Context::getOptions() { return options; }

  source::BufferId Context::loadFile(const std::string &path, const source::Range &range)
  {
    if (!std::filesystem::exists(path))
    {
      diagnosticEmitter.error(
        range,
        diagnostics::DiagnosticId::IO_PATH_DOES_NOT_EXISTS, path
      );
    }

    try
    {
      return bufferManager.loadFile(path);
    } catch (const RuntimeError &e)
    {
      diagnosticEmitter.error(range, e.what());
    }
  }
}
