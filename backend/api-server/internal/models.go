package models

type Role string

const (
	Follower Role = "follower"
	Trader Role = "trader"
)

type User struct {
    ID    uint   `gorm:"primarykey"`
    Name  string
    Email string `gorm:"uniqueIndex"`
    PasswordHash string
    OauthProvider string
    OauthId string
    EmailVerified bool `json:"email_verified"`
    VerificationToken string `json:"verification_token"`
    Role Role `json:"role"`
    TwoFactorEnabled bool `json:"two_factor_enabled"`
    TwoFactorSecret string `json:"two_factor_secret"`
}
