package lib

import (
	"log"
	"os"
)

const (
	LoggerLabelInfo    = "INFO"
	LoggerLabelWarning = "WARNING"
	LoggerLabelError   = "ERROR!"
	LoggerLabelFatal   = "FATAL!!"

	LoggerLabelHTTPError = "HTTP ERROR!"
)

var LibLogger = log.New(os.Stderr, "LIB LOGGER ", log.LstdFlags|log.Ltime)
