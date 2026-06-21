cmake_minimum_required(VERSION 3.20)

project(lambda_discipline)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

include(FetchContent)

FetchContent_Declare(
    fmt
    GIT_REPOSITORY https://github.com/fmtlib/fmt.git
    GIT_TAG 11.2.0
)

FetchContent_Declare(
    replxx
    GIT_REPOSITORY https://github.com/AmokHuginnsson/replxx.git
    GIT_TAG master
)

FetchContent_Declare(
    CLI11
    GIT_REPOSITORY https://github.com/CLIUtils/CLI11.git
    GIT_TAG v2.5.0
)

FetchContent_MakeAvailable(fmt)
FetchContent_MakeAvailable(replxx)
FetchContent_MakeAvailable(CLI11)

function(enable_warnings target)
    if (MSVC)
        target_compile_options(${target} PRIVATE
            /W4
            /permissive-
        )
    else ()
        target_compile_options(${target} PRIVATE
            -Wall
            -Wextra
            # -Wpedantic
            # -Wconversion
            # -Wshadow
            # -Wnon-virtual-dtor
            # -Wold-style-cast
            # -Woverloaded-virtual
        )
    endif ()
endfunction()

###### SOURCE ######

file(GLOB_RECURSE SOURCE_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/source/*.cpp
)

add_library(lbdSource STATIC
    ${SOURCE_SOURCE_FILES}
)

target_include_directories(lbdSource PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

###### DIAGNOSTICS ######

file(GLOB_RECURSE DIAGNOSTICS_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/diagnostics/*.cpp
)

add_library(lbdDiagnostics STATIC
    ${DIAGNOSTICS_SOURCE_FILES}
)

target_include_directories(lbdDiagnostics PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

target_link_libraries(lbdDiagnostics PUBLIC
    lbdSource
    fmt::fmt
)


###### TYPE ######

file(GLOB_RECURSE TYPE_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/type/*.cpp
)

add_library(lbdType STATIC
    ${TYPE_SOURCE_FILES}
)

target_include_directories(lbdType PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

###### FRONTEND ######

file(GLOB_RECURSE FRONTEND_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/frontend/*.cpp
)

add_library(lbdFrontend STATIC
    ${FRONTEND_SOURCE_FILES}
)

target_include_directories(lbdFrontend PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

target_link_libraries(lbdFrontend PUBLIC
    lbdDiagnostics
)

###### RUNTIME ######

file(GLOB_RECURSE RUNTIME_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/runtime/*.cpp
)

add_library(lbdRuntime STATIC
    ${RUNTIME_SOURCE_FILES}
)

target_include_directories(lbdRuntime PUBLIC
    ${PROJECT_SOURCE_DIR}/include
)

target_link_libraries(lbdRuntime PUBLIC
    lbdDiagnostics
)

###### MAIN EXECUTABLE ######

file(GLOB LBD_SOURCE_FILES CONFIGURE_DEPENDS
    ${PROJECT_SOURCE_DIR}/src/*.cpp
    ${PROJECT_SOURCE_DIR}/src/utils/terminal/*.cpp
    ${PROJECT_SOURCE_DIR}/src/cli/*.cpp
    ${PROJECT_SOURCE_DIR}/src/cli/commands/*.cpp
)

add_executable(lbd
    ${LBD_SOURCE_FILES}
)

target_include_directories(lbd PRIVATE
    ${PROJECT_SOURCE_DIR}/include
)

target_link_libraries(lbd PRIVATE
    lbdDiagnostics
    lbdFrontend
    lbdRuntime
    lbdType
    replxx
    CLI11::CLI11
)

if (CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_link_options(lbd PRIVATE
        -static-libstdc++
        -static-libgcc
    )
endif ()

###### WARNINGS ######

enable_warnings(lbdSource)
enable_warnings(lbdDiagnostics)
enable_warnings(lbdFrontend)
enable_warnings(lbdRuntime)
enable_warnings(lbd)

###### TESTS ######

enable_testing()

# add_executable(lexer_test tests/lexer_test.cc)
# target_link_libraries(lexer_test PRIVATE lbdFrontend)
# add_test(NAME lexer_test COMMAND lexer_test)