#include <lbd/REPL.hpp>
#include <lbd/cli/commands/REPLCommand.hpp>

namespace lbd::cli
{
  void REPLCommand::configure(CLI::App &command)
  {
    command.name("repl");
    command.description("Start an interactive Read-Eval-Print Loop (REPL)");
  }

  int REPLCommand::execute(Context &context)
  {
    repl::loop(context, context.getOptions().debug);
    return EXIT_SUCCESS;
  }
}