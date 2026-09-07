#include <lbd/Docs.hpp>
#include <lbd/cli/commands/DocsCommand.hpp>

namespace lbd::cli
{
  void DocsCommand::configure(CLI::App &command)
  {
    command.name("docs");
    command.description("Generate native function signatures for reference");
  }

  int DocsCommand::execute(Context &)
  {
    dumpDocs(std::cout);
    return EXIT_SUCCESS;
  }
}
