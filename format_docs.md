## TAGame.Default__PRI_TA
Player replication info object, one of these is made for each player
The ActorID is a player

## Engine.PlayerReplicationInfo:*
Player replication info for the player. This has the same actor as the `TAGame.Default__PRI_TA`
These attributes collect info about the player, such as their name, ping, etc

### Engine.PlayerReplicationInfo:Team
This is how to put a player on a team
The actor is a reference to `TAGame.Default__PRI_TA`
The attribute is an active actor that matches either a `Archetypes.Teams.Team0` or `Archetypes.Teams.Team1` actor

## Engine.Pawn:PlayerReplicationInfo
This relates a player to a car. The attribute is an active actor that matches the `TAGame.Default__PRI_TA` actor object
The actor ID is the Car archetype

## Archetypes.Car.Car_Default
This is an instance of a car
This only exists if a player selects a team

## Archetypes.Ball.Ball_Default
The actor for the ball

## Archetypes.Teams.Team0
The actor for team 0 (blue?)

## Archetypes.Teams.Team1
The actor for team 1 (orange?)

## TAGame.RBActor_TA:ReplicatedRBState
When a car moves, the actor id is of the Archetypes.Car.Car_Default