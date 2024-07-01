#!/bin/bash

# Set up environment variables
export PROJECTM_ROOT=/var/task/projectm
export GST_PLUGIN_PATH=/var/task/projectm/gstreamer-plugins/

# Execute the Lambda function handler
exec /usr/src/projectm_lambda/target/release/projectm_lambda "$@"
