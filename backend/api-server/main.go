package main

import (
	"copytrading/apiserver/internal/common/db"
	"copytrading/apiserver/internal/router"
)

func main() {
 	db.Init();
	db.Migrate()
	router.SetupRoutes()
	// app := fiber.New()
	// api := app.Group("/api")
	// auth := api.Group("/auth");

	// app.Get("/", func(c *fiber.Ctx) error {
	// 	return c.SendString("Hello, World!")
	// })


	// app.Listen(":3000")
}
