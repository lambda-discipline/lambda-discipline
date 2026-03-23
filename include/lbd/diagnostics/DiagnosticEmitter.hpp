#pragma once

#include <utility>
#include <vector>
#include <fmt/args.h>
#include <fmt/format.h>
#include <lbd/diagnostics/ContextFrame.hpp>
#include <lbd/diagnostics/Diagnostic.hpp>
#include <lbd/diagnostics/DiagnosticId.hpp>
#include <lbd/diagnostics/DiagnosticRenderer.hpp>
#include <lbd/diagnostics/Severity.hpp>

namespace lbd::diagnostics
{
    class DiagnosticEmitter
    {
    public:
        explicit DiagnosticEmitter(DiagnosticRenderer& diagnosticRenderer);

        void pushContext(source::Range range, std::string message);

        void popContext();

        template <typename... Args>
        [[noreturn]] void error(const source::Range& range, const DiagnosticId id, Args&&... args)
        {
            fmt::dynamic_format_arg_store<fmt::format_context> store;
            (store.push_back(std::forward<Args>(args)), ...);
            emit(Severity::ERROR, id, range, store);
            std::unreachable();
        }

        template <typename... Args>
        [[noreturn]] void warning(const source::Range& range, const DiagnosticId id, Args&&... args)
        {
            fmt::dynamic_format_arg_store<fmt::format_context> store;
            (store.push_back(std::forward<Args>(args)), ...);
            emit(Severity::WARNING, id, range, store);
        }

        void emit(Severity severity, DiagnosticId id, const source::Range& range,
                  const fmt::dynamic_format_arg_store<fmt::format_context>& arguments) const;

    private:
        DiagnosticRenderer& diagnosticRenderer;
        std::vector<ContextFrame> contextStack;
    };
}
