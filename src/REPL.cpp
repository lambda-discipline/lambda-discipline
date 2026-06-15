#include <filesystem>
#include <fstream>
#include <iostream>
#include <replxx.hxx>
#include <lbd/exceptions.h>
#include <lbd/REPL.hpp>
#include <lbd/ScopedOptionsOverride.hpp>
#include <lbd/frontend/lexer/Lexer.hpp>
#include <lbd/frontend/parser/Parser.hpp>
#include <lbd/runtime/interpreter.h>
#include <lbd/utils/terminal/Table.hpp>
#include <lbd/utils/terminal/Terminal.hpp>

#define ON_OR_OFF(val) ((val) ? "on " : "off")
#define LBD_HISTORY ".lbd_history"

// TODO: Option :t for display type information of a symbol.

namespace lbd::repl
{
  static void processLoadCommand(const std::string &argument, Context &context,
                                 std::optional<std::shared_ptr<runtime::Environment>> &sharedEnvironment)
  {
    const std::string &path = argument;

    // TODO: Should an already loaded file be loaded again. Current logic does not permit that.
    const source::BufferId bufferId = context.loadFile(path, {});
    const source::Buffer buffer = context.getBufferManager().getBuffer(bufferId);
    frontend::lexer::Lexer lexer(context, buffer);

    // const auto tokens = lexer.lex();
    // if (subOptions.debug)
    // {
    //     for (const auto& token : tokens)
    //     {
    //         subOptions.logger.debug(token);
    //     }
    // }

    auto astNodes = frontend::parser::Parser(context, lexer).parse();
    frontend::Program program(std::move(astNodes));

    // if (subOptions.debug)
    // {
    //     for (const auto& node : program.astNodes)
    //     {
    //         subOptions.logger.debug(node);
    //     }
    // }

    const std::optional<std::shared_ptr<runtime::Environment>> temporaryEnvironment = sharedEnvironment;
    // Merge loaded_env into shared_env
    if (const auto [loadedEnvironment, _, resultantOptions] = runtime::interpret(
      program, context, temporaryEnvironment); loadedEnvironment)
    {
      if (!sharedEnvironment)
      {
        sharedEnvironment = loadedEnvironment;
      } else
      {
        // Merge all Bindings from loaded_env into shared_env
        for (const auto &[first, second]: loadedEnvironment->table)
        {
          (*sharedEnvironment)->bind(first, second);
        }
      }
      if (resultantOptions.sideEffects) std::cout << std::endl;
    }

    context.getLogger().info("info: file loaded ", path);
  }

  static int computeParenthesisDepth(const std::string &data)
  {
    int depth = 0;
    for (const char c: data)
    {
      if (c == '(') depth++;
      else if (c == ')') depth = std::max(0, depth - 1);
    }
    return depth;
  }

  static std::string trim(const std::string &data)
  {
    size_t start = 0;
    while (start < data.size() && std::isspace(static_cast<unsigned char>(data[start]))) start++;
    size_t end = data.size();
    while (end > start && std::isspace(static_cast<unsigned char>(data[end - 1]))) end--;
    return data.substr(start, end - start);
  }

  void loop(Context &context, const bool debug)
  {
    utils::terminal::enableVirtualTerminal();
    utils::terminal::Colors colors{};
    replxx::Replxx rx;
    rx.set_max_history_size(1000);
    rx.history_load(LBD_HISTORY);

    ScopedOptionsOverride options(context);
    options.get().ownExpression = true;
    options.get().debug = debug;

    std::string line, buffer;
    size_t indentLevel = 0;
    std::optional<std::shared_ptr<runtime::Environment>> sharedGlobalEnvironment = std::nullopt;

    context.getLogger().info("Welcome to lambda-discipline REPL.\nType :quit to exit.");

    while (true)
    {
      try
      {
        // Prompt depends on whether we are continuing a buffer or not.
        std::string prompt = colors.cyan + std::string(buffer.empty() ? "\n>> " : ".. ") + colors.reset;
        for (size_t i = 0; i < indentLevel; i++) std::cout << "  ";

        char const *input = rx.input(prompt);
        if (!input) break; // EOF (C-d / C-z)
        line = input;
        if (!line.empty()) rx.history_add(line);

        // REPL arguments parsing
        {
          if (line == ":q" || line == ":quit" || line == ":exit")
          {
            context.getLogger().info("\nexiting REPL.");
            break;
          }

          if (line == ":c" || line == ":clear" || line == ":cls")
          {
#ifdef _WIN32
            std::system("cls");
#else
            std::system("clear");
#endif
            continue;
          }

          if (line == ":h" || line == ":help" || line == ":?")
          {
            std::cout << std::endl;
            utils::terminal::printTable(
              {"General Commands", "Argument", "Description"}, {
                {":q, :quit, :exit", "", "Exit the REPL"},
                {":c, :clear, :cls", "", "Clear the screen"},
                {":h, :help, :?", "", "Display this help message"},
                {":l, :load", "<filepath>", "Load file into REPL"},
                {":r, :reset", "", "Reset environment"},
                {":d, :debug", "", "Toggle debug mode"}
              }, colors.green);
            std::cout << std::endl;
            utils::terminal::printTable(
              {"Inspection Commands", "Argument", "Description"}, {
                {":e, :env", "", "Dump environment bindings"},
                {":force", "", "Force thunk evaluation on dump"}
              }, colors.green);
            std::cout << std::endl;
            utils::terminal::printTable(
              {"Options", "State", "Help"}, {
                {"debug", ON_OR_OFF(options.get().debug), "use :debug to toggle"},
                {
                  "force-on-env-dump", ON_OR_OFF(options.get().forceOnEnvironmentDump),
                  "use :force to toggle"
                }
              }, colors.green);
            continue;
          }

          if (line == ":env" || line == ":e")
          {
            std::cout << std::endl;
            if (sharedGlobalEnvironment)
            {
              // TODO: To vector can just accept context, forceOnEnvironmentDump
              //       can be extracted from the context.
              utils::terminal::printTable({"Symbol", "Thunk"},
                                          (*sharedGlobalEnvironment)->toVector(
                                            context, options.get().forceOnEnvironmentDump),
                                          colors.green);
            } else
            {
              utils::terminal::printTable({"Symbol", "Thunk"}, {{"Empty"}}, colors.green);
            }
            continue;
          }

          if (line == ":reset" || line == ":r")
          {
            sharedGlobalEnvironment.reset();
            continue;
          }

          if (line == ":debug" || line == ":d")
          {
            options.get().debug = !options.get().debug;
            continue;
          }

          if (line == ":force")
          {
            options.get().forceOnEnvironmentDump = !options.get().forceOnEnvironmentDump;
            continue;
          }

          if (line.rfind(":load ", 0) == 0)
          {
            processLoadCommand(line.substr(6), context, sharedGlobalEnvironment);
            continue;
          }

          if (line.rfind(":l ", 0) == 0)
          {
            processLoadCommand(line.substr(3), context, sharedGlobalEnvironment);
            continue;
          }
        }

        if (line.empty()) continue;

        std::string trimmed = trim(line);
        // Continue the line with trailing '`'.
        bool continuation = !trimmed.empty() && trimmed.back() == '`';
        if (continuation)
        {
          trimmed.pop_back();
        }
        buffer += trimmed + "\n";

        if (continuation)
        {
          // Indent level depends on whether the current line ends with '.' after trimming
          // and removing trailing '`'. Additionally, it also depends upon the current parenthesis depth.
          indentLevel = computeParenthesisDepth(buffer);
          trimmed = trim(trimmed);
          if (!trimmed.empty() && trimmed.back() == '.') indentLevel++;
          continue;
        }
        indentLevel = 0;
        line = buffer;
        buffer.clear();

        // Lex
        const source::BufferId bufferId = context.getBufferManager().createBuffer("<REPL>", line);
        const source::Buffer &sourceBuffer = context.getBufferManager().getBuffer(bufferId);
        frontend::lexer::Lexer lexer(context, sourceBuffer);

        // if (optionsValue.debug)
        // {
        //     for (const auto& tok : tokens)
        //     {
        //         optionsValue.logger.debug(tok);
        //     }
        // }

        // Parse
        auto astNodes = frontend::parser::Parser(context, lexer).parse();
        frontend::Program program(std::move(astNodes));

        // if (optionsValue.debug)
        // {
        //     optionsValue.logger.debug(program);
        // }

        // Interpret
        // TODO: Either remove ScopedOptionsOverride, or introduce subContext, or decouple options from context.
        context.getOptions() = options.get();
        const auto [globalEnvironment, value, resultantOptions] = runtime::interpret(
          program, context, sharedGlobalEnvironment);
        if (resultantOptions.sideEffects)
        {
          std::cout << std::endl;
        }
        std::cout << colors.green << "=> " << value << colors.reset << std::endl;
        sharedGlobalEnvironment = globalEnvironment;
      } catch (const ControlledExit &) {} catch (const std::exception &ex)
      {
        // TODO: This exits on error.
        context.getLogger().error("error: ", ex.what());
      }
    }

    rx.history_save(LBD_HISTORY);
  }
}
