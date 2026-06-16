#pragma once

#include <lbd/diagnostics/Diagnostic.hpp>
#include <lbd/source/BufferManager.hpp>

namespace lbd::diagnostics
{
  class DiagnosticRenderer
  {
  public:
    DiagnosticRenderer(std::ostream &outputStream, source::BufferManager &sourceManager);

    void render(const Diagnostic &diagnostic) const;

  private:
    std::ostream &outputStream;
    const source::BufferManager &sourceManager;
  };
}