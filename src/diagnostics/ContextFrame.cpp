#include <lbd/diagnostics/ContextFrame.hpp>

namespace lbd::diagnostics
{
  ContextFrame::ContextFrame(const source::Range &range, std::string message) : range(range),
    message(std::move(message)) {}

  source::Range ContextFrame::getRange() const { return range; }

  std::string ContextFrame::getMessage() const { return message; }
}