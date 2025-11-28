package router

import (
	authhandler "copytrading/apiserver/internal/handlers/authHandler"

	"github.com/gin-contrib/cors"
	"github.com/gofiber/fiber/v2"
)

func SetupRoutes()*fiber.App{
	app := fiber.New()

	app.Use(cors.New(cors.Config{
		AllowOrigins:     "*",
		AllowMethods:     "*",
		AllowHeaders:     "*",
		AllowCredentials: true,
	}))

	api:= app.Group("/api")
	authrouter := api.Group("/auth")
	{
		authrouter.Post("/login", authhandler.Login)
		authrouter.Post("/register", authhandler.Register)
	}

	return app
}


// func SetupRouter() *fiber.App {
// 	app := fiber.New()

// 	app.Use(cors.New(cors.Config{
// 		AllowOrigins:     "*",
// 		AllowMethods:     "*",
// 		AllowHeaders:     "*",
// 		AllowCredentials: true,
// 	}))

// 	app.Get("/ping", func(c *fiber.Ctx) error {
// 		return c.SendString("pong")
// 	})

// 	// webhooks
// 	webhooks := app.Group("/webhooks")

// 	appEvents := webhooks.Group("/app", middlewares.VerifyAppEventWebhook)
// 	{
// 		appEvents.Get("/metadata", appEventHandlers.HandleGetMetadata)
// 		appEvents.Post("/update-workflow-run-id", appEventHandlers.HandleUpdateWorkflowRunID)
// 		appEvents.Post("/update-provision-outputs", appEventHandlers.HandleUpdateProvisionOutputs)
// 		appEvents.Post("/update-project-setup-info", appEventHandlers.HandleUpdateProjectSetupInfo)
// 		appEvents.Post("/update-project-envs-metadata", appEventHandlers.HandleUpdateProjectEnvsMetadata)
// 		appEvents.Post("/new-activity", appEventHandlers.HandleNewActivity)
// 	}

// 	webhooks.Post("/:repoProviderKey", repoEventHandlers.HandleCodeEvents)

// 	// customer routes
// 	app.Get("/me", middlewares.EnsureCustomerJWT, handlers.GetMe)

// 	// info
// 	info := app.Group("/info", middlewares.EnsureCustomerJWT)
// 	{
// 		info.Get("/code-base-templates", infoHandlers.HandleGetCodeBaseTemplates)
// 		info.Get("/public-apps", infoHandlers.HandleGetPublicApps)
// 	}

// 	// projects
// 	projects := app.Group("/projects", middlewares.EnsureCustomerJWT)
// 	{
// 		projects.Post("/", handlers.HandlePostProject)
// 		projects.Get("/", handlers.ListProjects)

// 		project := projects.Group("/:projectName")
// 		{
// 			project.Get("/", handlers.HandleGetProject)
// 			project.Get("/activity-logs", handlers.HandleGetActivityLogs)
// 			project.Get("/workflow-jobs", handlers.HandleGetWorkflowJobs)
// 			project.Post("/rename", handlers.HandleRenameProject)

// 			branch := project.Group("/:branchName")
// 			{
// 				branch.Post("/update-metadata", branchHandlers.HandleUpdateMetadata)
// 				branch.Post("/set-secret-inheritance", branchHandlers.HandleSetSecretInheritance)
// 				branch.Post("/set-entrypoint", branchHandlers.HandleSetEntrypoint)

// 				branch.Post("/activate", branchHandlers.HandleActivateBranchConfig)
// 				branch.Post("/deactivate", branchHandlers.HandleDeactivateBranchConfig)
// 				branch.Post("/build-deploy", branchHandlers.HandleBuildDeploy)
// 				branch.Post("/rename", branchHandlers.HandleBranchRename)

// 				trigger := branch.Group("/trigger")
// 				{
// 					trigger.Post("/provision", triggerHandlers.HandleTriggerProvision)
// 				}

// 				env := branch.Group("/env")
// 				{
// 					env.Get("/", envHandlers.HandleGetEnv)
// 					env.Post("/set-vars", envHandlers.HandleEnvSetVars)
// 					env.Post("/delete-vars", envHandlers.HandleEnvDeleteVars)
// 					env.Post("/clear", envHandlers.HandleEnvClear)
// 				}

// 				component := branch.Group("/:componentName")
// 				{
// 					component.Post("/create", componentHandlers.HandleComponentCreate)
// 					component.Post("/delete", componentHandlers.HandleComponentDelete)
// 					component.Post("/set-config", componentHandlers.HandleComponentSetConfig)
// 					component.Post("/rename", componentHandlers.HandleComponentRename)

// 					componentEnv := component.Group("/env")
// 					{
// 						componentEnv.Post("/delete-vars", componentEnvHandlers.HandleEnvDeleteVars)
// 					}
// 				}

// 				apps := branch.Group("/apps")
// 				{
// 					apps.Get("/", branchAppHandlers.HandleGetApps)
// 					apps.Post("/install", branchAppHandlers.HandleAppInstall)
// 					apps.Post("/uninstall", branchAppHandlers.HandleAppUninstall)
// 				}
// 			}
// 		}
// 	}

// 	// auth
// 	app.Post("/login", authHandlers.Login)
// 	app.Post("/register", authHandlers.Register)

// 	return app
// }
