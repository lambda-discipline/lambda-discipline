#include <iomanip>
#include <ranges>
#include <lbd/diagnostics/DiagnosticRenderer.hpp>
#include <lbd/utils/terminal/Colors.hpp>
#include <lbd/utils/terminal/Terminal.hpp>

namespace lbd::diagnostics
{
  struct BoxChars
  {
    const char *topLeft;
    const char *bottomLeft;
    const char *horizontal;
    const char *vertical;
    const char *tee;
  };

  static BoxChars getBoxChars() noexcept
  {
    return utils::terminal::supportsUnicode() ? BoxChars{"┌", "└", "─", "│", "├"} : BoxChars{"+", "+", "-", "|", "+"};
  }

  DiagnosticRenderer::DiagnosticRenderer(std::ostream &outputStream, source::BufferManager &sourceManager)
    : outputStream(outputStream), sourceManager(sourceManager) {}

  void DiagnosticRenderer::render(const Diagnostic &diagnostic) const
  {
    /**
     *
     *        ERROR: Unexpected character `;`.
     *    ┌─ /home/nosferatu/Projects/lambda-discipline/dump/test.lbd:1..1
     *    │
     *  1 │ a; Number = 1
     *    │  ^
     *    │
     *    ├─ NOTE: While parsing `/home/nosferatu/Projects/lambda-discipline/dump/test.lbd`.
     *    └─ NOTE: While lexing `/home/nosferatu/Projects/lambda-discipline/dump/test.lbd`.
     *
     */

    const auto [topLeft, bottomLeft, horizontal, vertical, tee] = getBoxChars();
    const utils::terminal::Colors colors;

    outputStream << getSeverityColor(diagnostic.getSeverity(), colors) << severityToString(diagnostic.getSeverity())
        << colors.reset << ": " << diagnostic.getMessage() << '\n';

    // ReSharper disable once CppUseStructuredBinding
    const source::Location beginLocation = diagnostic.getRange().begin;
    // ReSharper disable once CppUseStructuredBinding
    const source::Location endLocation = diagnostic.getRange().end;
    const source::BufferId bufferId = beginLocation.bufferId;
    outputStream << "    " << topLeft << horizontal << " " << sourceManager.getBufferName(bufferId)
        << ':' << diagnostic.getRange() << '\n';

    // Source line.
    outputStream << "    " << vertical << '\n';
    const auto lineContent = sourceManager.getLine(beginLocation.bufferId, beginLocation.offset);
    outputStream << std::setw(3) << sourceManager.getRow(beginLocation.bufferId, beginLocation.offset)
        << " " << vertical << " " << lineContent << '\n';

    // Caret underline.
    const size_t startColumn = sourceManager.getColumn(beginLocation.bufferId, beginLocation.offset);
    const size_t endColumn = sourceManager.getColumn(endLocation.bufferId, endLocation.offset);
    const size_t caretOffset = startColumn - 1;
    const size_t caretLength = endColumn > startColumn ? endColumn - startColumn : 1;

    outputStream << "    " << vertical << " " << std::string(caretOffset, ' ')
        << std::string(caretLength, '^') << '\n';
    outputStream << "    " << vertical << '\n';

    // Notes (context stack).
    size_t i = 0;
    for (const auto &ctx: diagnostic.getContextStack())
    {
      i += 1;
      outputStream << "    " << (i < diagnostic.getContextStack().size() ? tee : bottomLeft);
      outputStream << horizontal << " " << getSeverityColor(Severity::NOTE, colors)
          << severityToString(Severity::NOTE) << colors.reset << ": " << ctx.getMessage() << '\n';
    }
  }
}