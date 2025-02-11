package main

import (
	"bufio"
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"math/rand"
	"net"
	"os"
	"slices"
	"strconv"
	"strings"
	"sync"
	"time"
	"unicode/utf8"
)

// [0, MaxNumber)
const MaxNumber = 100

func GenNumber() int64 {
	return rand.Int63n(MaxNumber)
}

func main() {
	var wg sync.WaitGroup
	commandChan := make(chan Command)

	wg.Add(1)
	go func() {
		defer wg.Done()
		defer close(commandChan)
		input := bufio.NewReader(os.Stdin)
		ended := false

	inputLoop:
		for !ended {
			input, err := input.ReadString('\n')
			if err != nil {
				if err != io.EOF {
					fmt.Printf("failed to read input(%T): %v\n", err, err)
					break
				}
				if input == "" {
					break
				}
				ended = true
			}

			input = strings.TrimRight(input, "\r\n")
			if input == "" {
				continue inputLoop
			}
			inputs := strings.Split(input, " ")
			command := inputs[0]
			args := inputs[1:]

			switch command {
			case "quit":
				break inputLoop
			case "help":
				commandChan <- &CommandHelp{}
			case "local":
				commandChan <- &CommandLocal{}
			case "serve":
				switch len(args) {
				case 0:
					commandChan <- &CommandServe{host: ":0", playerCount: 2}
				case 1:
					commandChan <- &CommandServe{host: args[0], playerCount: 2}
				case 2:
					playerCount, err := strconv.Atoi(args[1])
					if err != nil || playerCount < 1 {
						fmt.Println("invalid player count")
						continue inputLoop
					}

					commandChan <- &CommandServe{host: args[0], playerCount: uint(playerCount)}
				default:
					fmt.Println("invalid usage, try `help` for command usage")
					continue inputLoop
				}
			case "connect":
				if len(args) < 1 {
					fmt.Println("invalid usage, try `help` for command usage")
					continue inputLoop
				}
				var name string = fmt.Sprintf("%05d", rand.Int63n(99999))
				if len(args) >= 2 {
					if trimed := strings.TrimSpace(args[1]); len(trimed) > 0 {
						name = trimed
					}
				}
				commandChan <- &CommandConnect{host: args[0], name: name}
			case "show":
				commandChan <- &CommandShow{}
			case "stop":
				commandChan <- &CommandStop{}
			default:
				num, err := strconv.ParseInt(input, 10, 64)
				if err != nil {
					fmt.Printf("failed to parse %q as number, if you want input a command, try `help` to see command usage\n", input)
					continue inputLoop
				}

				commandChan <- &CommandNumber{number: num}
			}
		}
	}()

	var connection Connection = nil

	fmt.Println("use `local` to start a local session or type `help` to see other commands")
eventLoop:
	for {
		command, ok := <-commandChan
		if !ok {
			fmt.Println("exiting")
			break eventLoop
		}

		if connection != nil && connection.RemoteStopped() {
			connection = nil
		}

		switch command := command.(type) {
		case *CommandHelp:
			fmt.Printf("`help`: print this message\n" + "`quit`: exit this game\n" +
				"`local`: create a local game session\n" +
				"`serve [host] [player count]`: \n" +
				"    serve a game session, listen at [host],\n" +
				"    wait [player count] player then start the game,\n" +
				"    [host] if not provided, default to :0\n" +
				"    [player count] if not provided, default to 2\n" +
				"`connect <host> [name]`: \n" +
				"    joint to a game session hosted at <host>, as [name],\n" +
				"    if [name] is used by other player,\n" +
				"    the server would allocate you a new name\n" +
				"    [name] if not provided, default to five digit random number\n" +
				"[any number within -2^63 ~ 2^63-1]: guess the number\n" +
				"`show`: show the number and restart game\n" +
				"`stop`: stop or disconnect from current game session\n")
		case *CommandLocal:
			if connection != nil {
				fmt.Println("game session already running")
				continue eventLoop
			}
			connection = NewConnectionLocal()
		case *CommandServe:
			if connection != nil {
				fmt.Println("game session already running")
				continue eventLoop
			}
			conn, err := NewConnectionServe(command.host, command.playerCount)
			if err == nil {
				connection = conn
			}
		case *CommandConnect:
			if connection != nil {
				fmt.Println("game session already running")
				continue eventLoop
			}
			conn, err := NewConnectionRemote(command.host, command.name)
			if err == nil {
				connection = conn
			}
		case *CommandNumber:
			if connection == nil {
				fmt.Println("no game session running")
			} else {
				connection.Guess(command.number)
			}
		case *CommandShow:
			if connection == nil {
				fmt.Println("no game session running")
			} else {
				connection.Show()
			}
		case *CommandStop:
			if connection == nil {
				fmt.Println("no game session running")
			} else {
				connection.Stop()
				connection = nil
			}
		default:
			panic("unknown command")
		}
	}

	if connection != nil && !connection.RemoteStopped() {
		connection.Stop()
	}

	wg.Wait()
}

// command
type CommandId = uint8

type Command interface {
	Id() CommandId
}

type CommandHelp struct{}

func (s *CommandHelp) Id() CommandId {
	return 0
}

type CommandLocal struct{}

func (s *CommandLocal) Id() CommandId {
	return 0
}

type CommandServe struct {
	host        string
	playerCount uint
}

func (s *CommandServe) Id() CommandId {
	return 0
}

type CommandConnect struct {
	host string
	name string
}

func (s *CommandConnect) Id() CommandId {
	return 0
}

type CommandNumber struct {
	number int64
}

func (s *CommandNumber) Id() CommandId {
	return 0
}

type CommandShow struct{}

func (s *CommandShow) Id() CommandId {
	return 0
}

type CommandStop struct{}

func (s *CommandStop) Id() CommandId {
	return 0
}

// connection
type Connection interface {
	Guess(number int64)
	Show()
	Stop()

	RemoteStopped() bool
}

func PrintNewGame() {
	fmt.Println("new game started!")
}

func PrintCorrect() {
	fmt.Println("correct!")
}

func PrintLesser() {
	fmt.Println("too small!")
}

func PrintGreater() {
	fmt.Println("too big!")
}

func PrintStopped() {
	fmt.Println("game session stopped")
}

// connection local
type ConnectionLocal struct {
	current int64
}

func NewConnectionLocal() Connection {
	conn := ConnectionLocal{current: 0}
	conn.NewGame()
	return &conn
}

func (s *ConnectionLocal) NewGame() {
	PrintNewGame()
	s.current = GenNumber()
}

func (s *ConnectionLocal) Guess(number int64) {
	if number == s.current {
		PrintCorrect()
		s.NewGame()
	} else if number < s.current {
		PrintLesser()
	} else {
		PrintGreater()
	}
}

func (s *ConnectionLocal) Show() {
	fmt.Printf("the number is %d\n", s.current)
	s.NewGame()
}

func (s *ConnectionLocal) Stop() {
	PrintStopped()
}

func (s *ConnectionLocal) RemoteStopped() bool {
	return false
}

// connection server
type GuessResult uint8

const (
	Equal GuessResult = iota
	Lesser
	Greater
)

func (r GuessResult) String() string {
	switch r {
	case Equal:
		return "correct"
	case Lesser:
		return "too small"
	case Greater:
		return "too big"
	default:
		panic("unknown result")
	}
}

func failedToDo(action string, name string, err error) {
	fmt.Fprintf(os.Stderr, "failed to %s %s, err: %s\n", action, name, err.Error())
}

func failedToCheck(name string, err error) {
	failedToDo("check", name, err)
}

func failedToSend(name string, err error) {
	failedToDo("send", name, err)
}

func failedToRecv(name string, err error) {
	failedToDo("receive", name, err)
}

func unexpectedClose(whenReceiving string, err error) {
	fmt.Fprintf(
		os.Stderr,
		"unexpected disconnect when receiving %s, disconnect reason: %s\n",
		whenReceiving,
		err.Error())
}

type GameEventId = uint8

type GameEvent interface {
	Id() GameEventId
}

type GameEventPlayerJoin struct {
	name         string
	currentCount uint
	targetCount  uint
}

func (n *GameEventPlayerJoin) Id() GameEventId {
	return 0
}

type GameEventPlayerQuit struct {
	name string
}

func (n *GameEventPlayerQuit) Id() GameEventId {
	return 0
}

type GameEventGameNew struct{}

func (n *GameEventGameNew) Id() GameEventId {
	return 0
}

type GameEventGameStop struct {
	reason string
}

func (n *GameEventGameStop) Id() GameEventId {
	return 0
}

type GameEventGameGuess struct {
	playerName string
	number     int64
	result     GuessResult
}

func (n *GameEventGameGuess) Id() GameEventId {
	return 0
}

type GameEventGameShow struct {
	playerName string
	number     int64
}

func (n *GameEventGameShow) Id() GameEventId {
	return 0
}

func GameEventToString(e GameEvent) string {
	switch event := e.(type) {
	case *GameEventPlayerJoin:
		return fmt.Sprintf("%s joined(%d/%d)",
			event.name,
			event.currentCount,
			event.targetCount)
	case *GameEventPlayerQuit:
		return fmt.Sprintf("%s quit", event.name)
	case *GameEventGameNew:
		return fmt.Sprintf("new game started!")
	case *GameEventGameStop:
		return fmt.Sprintf("game stopped, %s", event.reason)
	case *GameEventGameGuess:
		return fmt.Sprintf(
			"%s guessed %d, it's %s!",
			event.playerName,
			event.number,
			event.result.String())
	case *GameEventGameShow:
		return fmt.Sprintf(
			"%s looked the number, it's %d!",
			event.playerName,
			event.number)
	default:
		panic("unknown event")
	}
}

type Player struct {
	eventChan chan<- GameEvent
}

type ConnectionServe struct {
	mutex       sync.Mutex
	playerCount uint
	players     map[string]Player
	current     int64

	stop context.CancelFunc
	wg   *sync.WaitGroup
}

type PacketOrErr struct {
	packet Packet
	err    error
}

func NewConnectionServe(host string, playerCount uint) (Connection, error) {
	ln, err := net.Listen("tcp", host)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to listen at %s, err: %s\n", host, err.Error())
		return nil, err
	}

	fmt.Printf("listening on %s\n", ln.Addr().String())

	ctx, stop := context.WithCancel(context.Background())
	var wg sync.WaitGroup

	conn := ConnectionServe{playerCount: playerCount,
		players: make(map[string]Player),
		current: 0,
		stop:    stop,
		wg:      &wg}
	hostEventChan := make(chan GameEvent, 10)
	conn.players["host"] = Player{eventChan: hostEventChan}
	wg.Add(1)
	go func() {
		defer wg.Done()
		defer delete(conn.players, "host")
		for {
			select {
			case event := <-hostEventChan:
				fmt.Println(GameEventToString(event))
			case <-ctx.Done():
				return
			}
		}
	}()

	handleClient := func(client net.Conn) {
		defer wg.Done()      // DEFER
		defer client.Close() // DEFER

		updateDeadline := func() {
			client.SetDeadline(time.Now().Add(time.Second * 5))
		}

		r := bufio.NewReader(client)
		w := bufio.NewWriter(client)

		disconnectReason := Unknown
		defer func() { // DEFER
			updateDeadline()
			err := SendPacket(w, &PacketDisconnect{reason: disconnectReason})
			if err != nil {
				failedToSend("disconnect", err)
			}
		}()

		updateDeadline()
		if err, reason := CheckMagicAndProtoVer(r); err != nil {
			failedToCheck("magic and protocol version", err)
			disconnectReason = reason
			return
		}

		updateDeadline()
		if err := SendMagicAndProtoVer(w); err != nil {
			failedToSend("magic and protocol version", err)
			return
		}

		updateDeadline()
		joinAs, err := ReadPacket(r)
		if err != nil {
			failedToRecv("join as", err)
			return
		}
		if err := MapDisconnectToErr(joinAs); err != nil {
			unexpectedClose("JoinAs", err)
			return
		}

		var playerName string
		if joinAs, ok := joinAs.(*PacketJoinAs); ok {
			playerName = joinAs.name
		} else {
			disconnectReason = ExpectJoinAfterHandshake
			return
		}

		eventChan := make(chan GameEvent, 10)
		broadcastEvent := func(event GameEvent) {
			conn.broadcastEvent(event, playerName)
		}

		joinSuccess := func() bool {
			conn.mutex.Lock()
			defer conn.mutex.Unlock()

			if len(conn.players) < int(conn.playerCount) {
				for {
					if _, ok := conn.players[playerName]; ok {
						playerName = strings.Join(
							[]string{playerName, fmt.Sprintf("%d", rand.Intn(10))},
							"")
					} else {
						break
					}
				}
				conn.players[playerName] = Player{eventChan: eventChan}

				conn.broadcastEventAll(&GameEventPlayerJoin{name: playerName,
					currentCount: uint(len(conn.players)),
					targetCount:  conn.playerCount})

				if conn.isPlayable() {
					conn.newGame()
					conn.broadcastEventAll(&GameEventGameNew{})
				}
				return true
			} else {
				return false
			}
		}()

		if joinSuccess {
			defer func() { // DEFER
				conn.mutex.Lock()
				defer conn.mutex.Unlock()

				isPlayableBefore := conn.isPlayable()
				delete(conn.players, playerName)
				broadcastEvent(&GameEventPlayerQuit{name: playerName})
				if isPlayableBefore {
					broadcastEvent(&GameEventGameStop{reason: "insufficient player"})
				}
			}()
		} else {
			disconnectReason = GameFull
			return
		}

		updateDeadline()
		SendPacket(w, &PacketJoinAs{name: playerName})

		client.SetDeadline(time.Time{})
		updateSendDeadline := func() {
			client.SetWriteDeadline(time.Now().Add(time.Second * 5))
		}

		recvChan := make(chan PacketOrErr, 10)
		wg.Add(1)
		go func() {
			// should stop when client close
			defer wg.Done()

			for {
				packet, err := ReadPacket(r)
				recvChan <- PacketOrErr{packet: packet, err: err}
				if err != nil {
					return
				}
			}
		}()

	serverLoop:
		for {
			select {
			case event := <-eventChan:
				updateSendDeadline()
				err := SendMsg(w, GameEventToString(event))
				if err != nil {
					failedToSend("event message", err)
					return
				}
			case packetOrErr := <-recvChan:
				if packetOrErr.err != nil {
					failedToRecv("play packet", packetOrErr.err)
					return
				}
				updateSendDeadline()
				switch packet := packetOrErr.packet.(type) {
				case *PacketGuess:
					conn.mutex.Lock()
					result, err := conn.guess(packet.number)
					if err != nil {
						SendMsg(w, err.Error())
						continue serverLoop
					}
					SendMsgFmt(w, "%s!", result.String())
					broadcastEvent(&GameEventGameGuess{
						playerName: playerName,
						number:     packet.number,
						result:     result})
					if result == Equal {
						conn.newGame()
						conn.broadcastEventAll(&GameEventGameNew{})
					}
					conn.mutex.Unlock()
				case *PacketShow:
					conn.mutex.Lock()
					result, err := conn.show()
					if err != nil {
						SendMsg(w, err.Error())
						continue serverLoop
					}
					SendMsgFmt(w, "it's %d!", result)
					broadcastEvent(&GameEventGameShow{
						playerName: playerName,
						number:     result})
					conn.newGame()
					conn.broadcastEventAll(&GameEventGameNew{})
					conn.mutex.Unlock()
				case *PacketDisconnect:
					fmt.Printf("%s disconnected, reason: %s\n",
						playerName,
						packet.reason.Error())
					return
				case *PacketJoinAs:
					disconnectReason = ExpectPlay
					return
				default:
					panic("unknown packet")
				}
			case <-ctx.Done():
				return
			}
		}
	}

	wg.Add(1)
	go func() {
		defer wg.Done()
		defer ln.Close()
		for {
			go func() {
				<-ctx.Done()
				ln.(*net.TCPListener).SetDeadline(time.Now())
			}()

			client, err := ln.Accept()
			if err != nil {
				if errors.Is(err, os.ErrDeadlineExceeded) {
					break
				}

				fmt.Fprintf(os.Stderr,
					"failed to accept connection, err: %s %T\n",
					err.Error(), err.(*net.OpError).Err)
				continue
			}

			wg.Add(1)
			go handleClient(client)
		}
	}()

	return &conn, nil
}

// require parent to lock
func (s *ConnectionServe) isPlayable() bool {
	return len(s.players) == int(s.playerCount)
}

func (s *ConnectionServe) newGame() {
	s.current = GenNumber()
}

func (s *ConnectionServe) guess(number int64) (GuessResult, error) {
	if !s.isPlayable() {
		return Equal, errors.New("game isn't running")
	}

	if number == s.current {
		return Equal, nil
	} else if number < s.current {
		return Lesser, nil
	} else {
		return Greater, nil
	}
}

func (s *ConnectionServe) show() (int64, error) {
	if !s.isPlayable() {
		return 0, errors.New("game isn't running")
	}

	return s.current, nil
}

func (s *ConnectionServe) broadcastEvent(event GameEvent, skip string) {
	for name, player := range s.players {
		if name == skip {
			continue
		}
		player.eventChan <- event
	}
}

func (s *ConnectionServe) broadcastEventAll(event GameEvent) {
	for _, player := range s.players {
		player.eventChan <- event
	}
}

func (s *ConnectionServe) Guess(number int64) {
	s.mutex.Lock()
	defer s.mutex.Unlock()

	result, err := s.guess(number)
	if err != nil {
		fmt.Fprintln(os.Stderr, err.Error())
		return
	}

	fmt.Printf("%s!\n", result.String())
	s.broadcastEvent(&GameEventGameGuess{playerName: "host",
		number: number,
		result: result},
		"host")
	if result == Equal {
		s.newGame()
		s.broadcastEventAll(&GameEventGameNew{})
	}
}

func (s *ConnectionServe) Show() {
	s.mutex.Lock()
	defer s.mutex.Unlock()

	result, err := s.show()
	if err != nil {
		fmt.Fprintln(os.Stderr, err.Error())
		return
	}

	s.broadcastEvent(&GameEventGameShow{playerName: "host",
		number: result},
		"host")
	fmt.Printf("the number is %d\n", result)
	s.newGame()
	s.broadcastEventAll(&GameEventGameNew{})
}

func (s *ConnectionServe) Stop() {
	s.mutex.Lock()

	fmt.Println("stopping")

	s.broadcastEvent(&GameEventGameStop{reason: "host stopping"}, "host")
	s.stop()

	s.mutex.Unlock()

	s.wg.Wait()
	PrintStopped()
}

func (s *ConnectionServe) RemoteStopped() bool {
	// the game session wouldn't be stopped by any client
	return false
}

// connection client
type RemoteCommandId uint8

type RemoteCommand interface {
	Id() RemoteCommandId
}

type RemoteCommandGuess struct {
	number int64
}

func (c *RemoteCommandGuess) Id() RemoteCommandId {
	return 0
}

type RemoteCommandShow struct{}

func (c *RemoteCommandShow) Id() RemoteCommandId {
	return 0
}

type RemoteCommandStop struct{}

func (c *RemoteCommandStop) Id() RemoteCommandId {
	return 0
}

type ConnectionRemote struct {
	cmdChan chan<- RemoteCommand

	wg      *sync.WaitGroup
	stopped bool
}

func NewConnectionRemote(host string, joinAs string) (Connection, error) {
	server, err := net.Dial("tcp", host)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to connect to %s, err: %s\n", host, err.Error())
		return nil, err
	}

	cmdChan := make(chan RemoteCommand)

	var wg sync.WaitGroup
	conn := &ConnectionRemote{cmdChan: cmdChan, wg: &wg, stopped: false}

	wg.Add(1)
	go func() {
		defer func() { conn.stopped = true }()
		defer wg.Done()
		defer server.Close() // DEFER

		updateDeadline := func() {
			server.SetDeadline(time.Now().Add(time.Second * 5))
		}
		updateSendDeadline := func() {
			server.SetWriteDeadline(time.Now().Add(time.Second * 5))
		}

		r := bufio.NewReader(server)
		w := bufio.NewWriter(server)

		disconnectReason := Unknown
		defer func() { // DEFER
			updateSendDeadline()
			err := SendPacket(w, &PacketDisconnect{reason: disconnectReason})
			if err != nil {
				failedToSend("disconnect", err)
			}
		}()

		updateDeadline()
		if err := SendMagicAndProtoVer(w); err != nil {
			failedToSend("magic and protocol version", err)
			return
		}

		updateDeadline()
		if err, reason := CheckMagicAndProtoVer(r); err != nil {
			failedToCheck("magic and protocol version", err)
			disconnectReason = reason
			return
		}

		updateDeadline()
		err := SendPacket(w, &PacketJoinAs{name: joinAs})
		if err != nil {
			failedToSend("join as", err)
			return
		}

		updateDeadline()
		joinAs, err := ReadPacket(r)
		if err != nil {
			failedToRecv("join as result", err)
			return
		}
		if err := MapDisconnectToErr(joinAs); err != nil {
			unexpectedClose("JoinAs", err)
		}

		var playerName string
		if joinAs, ok := joinAs.(*PacketJoinAs); ok {
			playerName = joinAs.name
		} else {
			disconnectReason = ExpectJoinAsResponse
			return
		}

		fmt.Printf("joined as %s\n", playerName)

		server.SetDeadline(time.Time{})
		server.SetReadDeadline(time.Time{})

		recvChan := make(chan PacketOrErr, 10)
		wg.Add(1)
		go func() {
			defer wg.Done()

			for {
				packet, err := ReadPacket(r)
				recvChan <- PacketOrErr{packet: packet, err: err}
				if err != nil {
					return
				}
			}
		}()

	clientLoop:
		for {
			select {
			case cmd := <-cmdChan:
				updateSendDeadline()
				switch cmd := cmd.(type) {
				case *RemoteCommandGuess:
					if err := SendPacket(w, &PacketGuess{number: cmd.number}); err != nil {
						failedToSend("guess", err)
						return
					}
				case *RemoteCommandShow:
					if err := SendPacket(w, &PacketShow{}); err != nil {
						failedToSend("show", err)
						return
					}
				case *RemoteCommandStop:
					disconnectReason = Normal
					break clientLoop
				}
			case packetOrErr := <-recvChan:
				if packetOrErr.err != nil {
					failedToRecv("play packet", packetOrErr.err)
					return
				}
				if packet, ok := packetOrErr.packet.(*PacketMsg); ok {
					fmt.Printf("[Server] %s\n", packet.msg)
				} else {
					disconnectReason = ExpectPlay
					return
				}
			}
		}
	}()

	return conn, nil
}

func (r *ConnectionRemote) Guess(number int64) {
	r.cmdChan <- &RemoteCommandGuess{number: number}
}

func (r *ConnectionRemote) Show() {
	r.cmdChan <- &RemoteCommandShow{}
}

func (r *ConnectionRemote) Stop() {
	r.cmdChan <- &RemoteCommandStop{}
	r.wg.Wait()
	PrintStopped()
}

func (r *ConnectionRemote) RemoteStopped() bool {
	return r.stopped
}

// protocol
var Magic = []byte{0x36, 0xe3, 0x54, 0x25}
var ProtocolVersion = []byte{0, 0, 0, 1}

type PacketId uint8

const (
	Disconnect PacketId = iota
	JoinAs
	Guess
	Show
	Msg
)

type Packet interface {
	Id() PacketId
}

type DisconnectReason uint8

const (
	Normal DisconnectReason = iota
	Unknown
	// handshake
	InvalidMagic
	VersionMismatch
	// join
	ExpectJoinAfterHandshake
	ExpectJoinAsResponse
	GameFull
	// playing
	ExpectPlay
)

func (r DisconnectReason) Error() string {
	switch r {
	case Normal:
		return "normal"
	case Unknown:
		return "unknown"
	case InvalidMagic:
		return "invalid magic"
	case VersionMismatch:
		return "mismatched protocol version"
	case ExpectJoinAfterHandshake:
		return "expect join after handshake"
	case ExpectJoinAsResponse:
		return "expect join as response after send join"
	case GameFull:
		return "game session already full"
	case ExpectPlay:
		return "expect play packet after join"
	default:
		panic("unknown reason")
	}
}

type PacketDisconnect struct {
	reason DisconnectReason
}

func (s *PacketDisconnect) Id() PacketId {
	return Disconnect
}

type PacketJoinAs struct {
	name string
}

func (s *PacketJoinAs) Id() PacketId {
	return JoinAs
}

type PacketGuess struct {
	number int64
}

func (s *PacketGuess) Id() PacketId {
	return Guess
}

type PacketShow struct{}

func (s *PacketShow) Id() PacketId {
	return Show
}

type PacketMsg struct {
	msg string
}

func (s *PacketMsg) Id() PacketId {
	return Msg
}

func encodeString(str string) []byte {
	strBuf := []byte(str)
	strLen := binary.AppendUvarint([]byte{}, uint64(len(strBuf)))
	return slices.Concat(strLen, strBuf)
}

func readString(r *bufio.Reader) (string, error) {
	strLen, err := binary.ReadUvarint(r)
	if err != nil {
		return "", err
	}

	strBuf := make([]byte, strLen)
	if _, err = io.ReadFull(r, strBuf); err != nil {
		return "", err
	}
	if !utf8.Valid(strBuf) {
		return "", fmt.Errorf("name %x isn't valid UTF-8", strBuf)
	}

	return string(strBuf), nil
}

func sendAll(w *bufio.Writer, buf []byte) error {
	_, err := w.Write(buf)
	if err != nil {
		return err
	}
	err = w.Flush()
	return err
}

func SendPacket(w *bufio.Writer, packet Packet) error {
	buf := []byte{byte(packet.Id())}

	switch packet := packet.(type) {
	case *PacketDisconnect:
		buf = append(buf, byte(packet.reason))
	case *PacketJoinAs:
		buf = slices.Concat(buf, encodeString(packet.name))
	case *PacketGuess:
		var err error
		buf, err = binary.Append(buf, binary.BigEndian, packet.number)
		if err != nil {
			return err
		}
	case *PacketShow:
	case *PacketMsg:
		buf = slices.Concat(buf, encodeString(packet.msg))
	default:
		panic("unknown packet")
	}

	return sendAll(w, buf)
}

func SendMsg(w *bufio.Writer, msg string) error {
	return SendPacket(w, &PacketMsg{msg: msg})
}

func SendMsgFmt(w *bufio.Writer, msg string, a ...any) error {
	return SendMsg(w, fmt.Sprintf(msg, a...))
}

func ReadPacket(r *bufio.Reader) (Packet, error) {
	id, err := r.ReadByte()
	if err != nil {
		return nil, err
	}

	switch id {
	case byte(Disconnect):
		reason, err := r.ReadByte()
		if err != nil {
			return nil, err
		}

		return &PacketDisconnect{reason: DisconnectReason(reason)}, nil
	case byte(JoinAs):
		name, err := readString(r)
		if err != nil {
			return nil, err
		}

		return &PacketJoinAs{name: name}, nil
	case byte(Guess):
		var number int64
		err := binary.Read(r, binary.BigEndian, &number)
		if err != nil {
			return nil, err
		}
		return &PacketGuess{number: number}, nil
	case byte(Show):
		return &PacketShow{}, nil
	case byte(Msg):
		msg, err := readString(r)
		if err != nil {
			return nil, err
		}

		return &PacketMsg{msg: msg}, nil
	default:
		return nil, fmt.Errorf("unknown packet: %x", id)
	}
}

func MapDisconnectToErr(packet Packet) error {
	if packet, ok := packet.(*PacketDisconnect); ok {
		return packet.reason
	} else {
		return nil
	}
}

func CheckMagicAndProtoVer(r *bufio.Reader) (error, DisconnectReason) {
	buf := make([]byte, 8)

	if _, err := io.ReadFull(r, buf); err != nil {
		return err, Unknown
	}

	if !slices.Equal(buf[:4], Magic) {
		return errors.New("invalid magic"), InvalidMagic
	}

	if !slices.Equal(buf[4:], ProtocolVersion) {
		return errors.New("mismatched version"), VersionMismatch
	}

	return nil, Unknown
}

func SendMagicAndProtoVer(w *bufio.Writer) error {
	return sendAll(w, slices.Concat(Magic, ProtocolVersion))
}
