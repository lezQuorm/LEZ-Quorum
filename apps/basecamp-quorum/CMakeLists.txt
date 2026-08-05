cmake_minimum_required(VERSION 3.14)
project(QuorumUiPlugin LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

logos_module(
    NAME quorum_ui
    REP_FILE src/quorum_ui.rep
    SOURCES
        src/quorum_ui_backend.h
        src/quorum_ui_backend.cpp
    INCLUDE_DIRS
        src
)
