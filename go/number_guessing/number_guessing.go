package main

import (
	"fmt"
	"math/rand"
	"strconv"
	"sync"
)

// exclusive
const MaxNumber = 10

func main() {
	var wg sync.WaitGroup
	commandChan := make(chan Command, 10)

	wg.Add(1)
	go func() {
		defer wg.Done()
	readInput:
		for {
			var command string

			_, err := fmt.Scanf("%s", &command)
			if err != nil {
				if err.Error() != "unexpected newline" {
					fmt.Printf("failed to read input(%T): %v\n", err, err)
					commandChan <- CommandQuit{}
					break
				}
				continue readInput
			}

			switch command {
			case "help":
				commandChan <- CommandHelp{}
			case "new":
				commandChan <- CommandNew{}
			case "show":
				commandChan <- CommandShow{}
			case "quit":
				commandChan <- CommandQuit{}
				break readInput
			default:
				num, err := strconv.ParseInt(command, 10, 64)
				if err != nil {
					fmt.Println("failed to parse input as number, if you want input a command, try `help` to see command usage")
					continue readInput
				}

				commandChan <- CommandNumber{number: num}
			}
		}
	}()

	var gameState GameState = GameStateIdle{}

	fmt.Println("use `new` to start a game or type `help` to see other commands")
eventLoop:
	for {
		command := <-commandChan
		switch command := command.(type) {
		case CommandHelp:
			fmt.Printf("    `help`: print this message\n" +
				"    `new`: start a new game\n" +
				"    [any number within -2^63 ~ 2^63-1]: guess the number\n" +
				"    `show`: just show the number\n" +
				"    `quit`: exit this game\n")
		case CommandNew:
			gameState = GameStatePlaying{number: int64(rand.Float64() * MaxNumber)}

			fmt.Println("game started!")
			fmt.Println("input your guess: ")
		case CommandNumber:
			state, playing := gameState.(GameStatePlaying)
			if !playing {
				fmt.Println("please start a game first")
				continue eventLoop
			}

			if command.number == state.number {
				fmt.Printf("you guessed it, it's exactly %d!\n", command.number)
				fmt.Println("game end")
				gameState = GameStateIdle{}
			} else if command.number > state.number {
				fmt.Println("too high")
			} else {
				fmt.Println("too low")
			}
		case CommandShow:
			state, playing := gameState.(GameStatePlaying)
			if !playing {
				fmt.Println("please start a game first")
				continue eventLoop
			}

			fmt.Printf("the number is %d\n", state.number)
			fmt.Println("game end")
			gameState = GameStateIdle{}

		case CommandQuit:
			fmt.Println("exiting")
			break eventLoop
		}
	}

	wg.Wait()
}

// command
type CommandId = uint8

const (
	Help CommandId = iota
	New
	Number
	Show
	Quit
)

type Command interface {
	Id() CommandId
}

type CommandHelp struct {
}

func (s CommandHelp) Id() CommandId {
	return Help
}

type CommandNew struct {
}

func (s CommandNew) Id() CommandId {
	return New
}

type CommandNumber struct {
	number int64
}

func (s CommandNumber) Id() CommandId {
	return Number
}

type CommandShow struct {
}

func (s CommandShow) Id() CommandId {
	return Show
}

type CommandQuit struct {
}

func (s CommandQuit) Id() CommandId {
	return Quit
}

// game state
type GameStateId = uint8

const (
	Idle GameStateId = iota
	Playing
)

type GameState interface {
	Id() GameStateId
}

type GameStateIdle struct {
}

func (s GameStateIdle) Id() GameStateId {
	return Idle
}

type GameStatePlaying struct {
	number int64
}

func (s GameStatePlaying) Id() GameStateId {
	return Playing
}
