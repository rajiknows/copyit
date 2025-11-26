package env

import (
	"log"
	"os"
)


var (
	DATABASE_URL         string
)

func init() {
	log.Println("env init from", os.Environ())
	DATABASE_URL = GetEnvVar("DATABASE_URL", false)
	log.Println("DATABASE URL", DATABASE_URL)
}

func GetEnvVar(key string, required bool) string {
	val, exists := os.LookupEnv(key)
	log.Println("GetEnvVar", key, val, exists)
	if !exists && required {
		envVarNotPresentPanic(key)
	}

	return val
}

func envVarNotPresentPanic(key string) {
	log.Fatalf("'%s' env var not present.", key)
}
