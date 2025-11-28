package db

import (
	models "copytrading/apiserver/internal"
	"copytrading/apiserver/internal/env"
	"log"

	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

var DB *gorm.DB

func Init() {
	var err error
	DB, err = gorm.Open(postgres.Open(env.DATABASE_URL), &gorm.Config{
		TranslateError:                           true,
		DisableForeignKeyConstraintWhenMigrating: true,
		IgnoreRelationshipsWhenMigrating:         true,
	})
	if err != nil {
		log.Fatal("Failed to connect to the database:", err)
	}
}

func Migrate(){
	DB.AutoMigrate(&models.User{})
}
